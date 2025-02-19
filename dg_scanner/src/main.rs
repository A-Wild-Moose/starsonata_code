use std::sync::{Arc, atomic::{AtomicBool, Ordering}};
use std::fs::{File, metadata};
use std::io::{BufReader, BufWriter, Write};
use std::collections::HashMap;
use pcap::{Device, Capture};
use regex::{Regex};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
struct DgLevel {
    id: String,
    galaxy: String,
    level: String,
    guards: String,
    boss: String
}

impl DgLevel {
    fn new(a: &str) -> (String, Self) {
        static RE_DG: Lazy<Regex> = Lazy::new(|| Regex::new(
            r"DG (?<dg_gal>[[:word:] ]*) (?<dg_level>[0-9]{1,2}\.[0-9]+[A-Z]?)"
        ).unwrap());
        let caps = RE_DG.captures(a).expect("Unable to parse the DG name");

        // this SHOULD be the first match, which should be the level we are actually entering
        let galaxy = caps.name("dg_gal").unwrap().as_str();
        let level = caps.name("dg_level").unwrap().as_str();
        let (_, mut id) = level.split_once(".").unwrap();

        // Remove any characters
        id = id.trim_end_matches(&['A', 'B', 'C', 'D']);

        println!("New DG Level - galaxy: {} level: {} id: {}", galaxy, level, id);

        let mut data = Self {
            id: format!("{} {}", galaxy, id).to_string(),
            galaxy: galaxy.to_string(),
            level: level.to_string(),
            guards: "?".to_string(),
            boss: "?".to_string(),
        };

        // handle the ships now
        static RE_SHIPS: Lazy<Regex> = Lazy::new(|| Regex::new(
            r"DX[0-9]{1,5}\u0000(?s:.*?)\u0000(?<ship>[[:word:]\. ]*)\u0000(Light Fighter|Heavy Fighter|Support Freighter|Capital Ship|Organic)"
        ).unwrap());
        
        let mut caps_ship = RE_SHIPS.captures_iter(a);

        // get first ship
        let (_, [ship, _]) = caps_ship.next().unwrap().extract();

        data.guards = ship.to_string();

        for cap in caps_ship {
            let (_, [ship, _]) = cap.extract();

            if ship == data.guards {
                // pass
            } else { // ship does not match existing guard
                if data.boss == "?" {
                    data.boss = ship.to_string();
                } else if ship != data.boss {
                    panic!("Ship does not match either boss or guard");
                } else {
                    data.boss = data.guards;
                    data.guards = ship.to_string();
                }
            }
        }

        if data.boss == "?" {
            println!("\t{}", data.guards);
        } else {
            println!("\t{}, {}", data.guards, data.boss);
        }

        (format!("{} {}", galaxy, level), data)
    }
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
        let packet = cap.next_packet().unwrap();
        let data = String::from_utf8_lossy(packet.data);

        static RE_META: Lazy<Regex> = Lazy::new(|| Regex::new(r"[\x00-\x1F]DG ").unwrap());
        if RE_META.is_match(&data) {
            // curr_dg = parse_dg(&data);
            dg_meta_packet.push_str(&data);
            println!("Starting galaxy data capture");
        }
        if dg_meta_packet != "" {
            dg_meta_packet.push_str(&data);
        }
        // if data.contains(format!("Entering DG {}", curr_dg).as_str()) {
        if data.contains("Entering DG ") {
            // TODO might need to update this to include data packet up to the Entering DG part
            dg_meta_packet.push_str(&data);

            write!(f, "{}\n\n", dg_meta_packet).expect("Unable to write debug log");

            let (gal, dg_level_data) = DgLevel::new(&dg_meta_packet);
            let _ = dg_data.insert(gal, dg_level_data);
            dg_meta_packet = "".to_string();
        }
    }

    let file = File::create(raw_path).unwrap();
    let writer = BufWriter::new(file);

    let _ = serde_json::to_writer(writer, &dg_data);
}
