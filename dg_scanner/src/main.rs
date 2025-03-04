use std::sync::{Arc, atomic::{AtomicBool, Ordering}};
use std::fs::{File, metadata};
use std::io::{BufReader, BufWriter, Write};
use std::collections::HashMap;
use pcap::{Device, Capture};
use regex::{Regex};
use once_cell::sync::Lazy;

use utils::device::get_pcap_capture;
use utils::dg_level::DgLevel;
mod utils;





fn main_() {
    // use std::fs;

    // let data = fs::read_to_string("raw/debug.log").expect("unable to read file");

    // // let re = Regex::new(r"DX[0-9]{1,5}\x00(?s:.*?)\x00(?<ship>[[:word:]\. ]*)\u0000(Light Fighter|Heavy Fighter|Support Freighter|Capital Ship|Organic)").unwrap();
    // let re = Regex::new(r"[\x00-\x1f](?<ship>[[:word:]'\. ]*)\u0000(Light Fighter|Heavy Fighter|Support Freighter|Capital Ship|Organic)").unwrap();
    // for cap in re.captures_iter(&data){
    //     let (_, [c1, c2]) = cap.extract();
    //     println!("{}", c1);
    // }
    let cap = get_pcap_capture();
    // println!("{:?}", main_device.desc);
}

fn main() {
    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();

    ctrlc::set_handler(move || {
        r.store(false, Ordering::SeqCst);
    }).expect("Error setting Ctrl-C handler");

    let mut main_device = Device::lookup().unwrap().unwrap();
    let devices = Device::list().unwrap();

    for dev in devices.iter(){
        if dev.desc.clone().unwrap().contains("Mullvad Tunnel") {
            main_device = dev.clone();
        }
    }

    // capture device
    let mut cap = Capture::from_device(main_device).unwrap().open().unwrap();
    
    // set the filters on the packat reading
    let _ = cap.filter(
        "src host 51.222.248.34", true
    );

    // save file
    let raw_path = "raw/raw_dgs_kd.json";
    let mut dg_data: HashMap<String, DgLevel> = if metadata(raw_path).is_ok() {
        let file = File::open(raw_path).unwrap();
        let reader = BufReader::new(file);

        serde_json::from_reader(reader).unwrap()
    } else {
        HashMap::new()
    };

    // debug log
    let f = File::create("raw/debug.log").expect("Unable to create debug log file");
    let mut f = BufWriter::new(f);

    // let mut curr_dg: String = "".to_string();
    let mut dg_meta_packet: String = "".to_owned();

    while running.load(Ordering::SeqCst) {
        let Ok(packet) = cap.next_packet() else {
            continue;
        };
        let data = String::from_utf8_lossy(packet.data);

        static RE_META: Lazy<Regex> = Lazy::new(|| Regex::new(r"[\x00-\x1F]DG ").unwrap());
        if let Some(m) = RE_META.find(&data) {
            dg_meta_packet.push_str(&data[m.start()..]);
        }
        if dg_meta_packet != "" {
            dg_meta_packet.push_str(&data);
        }
        if let Some(i1) = data.find("Entering DG ") {
            dg_meta_packet.push_str(&data[0..i1]);

            write!(f, "{}\n\n", dg_meta_packet).expect("Unable to write debug log");

            let (gal, dg_level_data) = DgLevel::new(&dg_meta_packet);
            // insert the data into the map. If it already exists, handle checking if we are actually updating an empty DG
            if let Some(val) = dg_data.get(&gal) {
                if val.guards == "?" {
                    let _ = dg_data.insert(gal, dg_level_data);
                }
            } else {
                let _ = dg_data.insert(gal, dg_level_data);
            }
            // let _ = dg_data.insert(gal, dg_level_data);
            dg_meta_packet = "".to_string();
        }
    }

    let file = File::create(raw_path).unwrap();
    let writer = BufWriter::new(file);

    let _ = serde_json::to_writer(writer, &dg_data);
}
