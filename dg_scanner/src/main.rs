use std::sync::{Arc, atomic::{AtomicBool, Ordering}};
use std::fs::File;
use std::io::{BufReader, BufWriter, Write};
use std::collections::HashMap;
use regex::{Regex};
use once_cell::sync::Lazy;

use polars::prelude::*;

use utils::device::get_pcap_capture;
use utils::dg_data::{DgLevel, DgData};
mod utils;


struct DgData_ {
    name: Vec<String>,
    id: Vec<String>,
    galaxy: Vec<String>,
    level: Vec<String>,
    guards: Vec<String>,
    boss: Vec<String>
}

impl DgData_ {
    fn new() -> Self{
        Self {
            name: Vec::new(),
            id: Vec::new(),
            galaxy: Vec::new(),
            level: Vec::new(),
            guards: Vec::new(),
            boss: Vec::new()
        }
    }

    fn add_level(&mut self, k: &String, v: &DgLevel) {
        self.name.push(k.to_string());
        self.id.push(v.id.clone());
        self.galaxy.push(v.galaxy.clone());
        self.level.push(v.level.clone());
        self.guards.push(v.guards.clone());
        self.boss.push(v.boss.clone());
    }
}





fn main_() {
    // use std::fs;

    // let data = fs::read_to_string("raw/debug.log").expect("unable to read file");

    // // let re = Regex::new(r"DX[0-9]{1,5}\x00(?s:.*?)\x00(?<ship>[[:word:]\. ]*)\u0000(Light Fighter|Heavy Fighter|Support Freighter|Capital Ship|Organic)").unwrap();
    // let re = Regex::new(r"[\x00-\x1f](?<ship>[[:word:]'\. ]*)\u0000(Light Fighter|Heavy Fighter|Support Freighter|Capital Ship|Organic)").unwrap();
    // for cap in re.captures_iter(&data){
    //     let (_, [c1, c2]) = cap.extract();
    //     println!("{}", c1);
    // }
    // let cap = get_pcap_capture();
    // println!("{:?}", main_device.desc);

    // Converting json store to dataframe
    let file = File::open("raw/raw_dgs_kd.json").unwrap();
    let reader = BufReader::new(file);
    let dg_data: HashMap<String, DgLevel> = serde_json::from_reader(reader).unwrap();

    let mut dg_data_2: DgData_ = DgData_::new();

    for (k, v) in dg_data.iter() {
        dg_data_2.add_level(k, v);
    }

    let mut df = DataFrame::new(
        vec![
            Column::new("Name".into(), dg_data_2.name),
            Column::new("Id".into(), dg_data_2.id),
            Column::new("Galaxy".into(), dg_data_2.galaxy),
            Column::new("Level".into(), dg_data_2.level),
            Column::new("Guard".into(), dg_data_2.guards),
            Column::new("Boss".into(), dg_data_2.boss)
        ]
    ).unwrap();

    df = df.sort(
        ["Id", "Name"],
        SortMultipleOptions::new()
            .with_order_descending_multi([false, true])
    ).unwrap();

    // let mut file = std::fs::File::open("raw/raw_dgs_kd.json").unwrap();
    // let df = JsonReader::new(&mut file).finish().unwrap();

    println!("{:?}", df);

}

fn main() {
    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();

    ctrlc::set_handler(move || {
        r.store(false, Ordering::SeqCst);
    }).expect("Error setting Ctrl-C handler");

    let mut cap = get_pcap_capture();

    // dg data store, with the save file
    let mut dg_data = DgData::new("raw/raw_dgs_kd.json");

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

            // update the dg data store
            dg_data.update(&dg_meta_packet);

            // reset the meta packet to empty for the next dg level
            dg_meta_packet = "".to_string();
        }
    }

    dg_data.store();
}
