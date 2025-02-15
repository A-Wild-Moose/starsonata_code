use std::sync::{Arc, atomic::{AtomicBool, Ordering}};
use std::fs::{File, metadata};
use std::io::{BufReader, BufWriter};
use std::collections::HashMap;
use pcap::{Device, Capture};
use regex::{Regex};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use serde_json::Result;

#[derive(Serialize, Deserialize)]
struct DgLevel {
    id: String,
    galaxy: String,
    level: String,
    guards: String,
    boss: String
}

impl DgLevel {
    fn new(a: &str) -> (str, Self) {
        static RE_DG: Lazy<Regex> = Lazy::new(|| Regex::new(
            r"DG (?<dg_gal>[[:word:] ]*) (?<dg_level>[0-9]{1,2}\.[0-9]+[A-Z]?)"
        ).unwrap());
        let caps = RE_DG.captures(a).expect("Unable to parse the DG name");

        // this SHOULD be the first match, which should be the level we are actually entering
        let galaxy = caps.name("dg_gal").unwrap().as_str();
        let level = caps.name("dg_level").unwrap().as_str();
        let [_, id] = level.split_once(".").unwrap();

        let mut data = Self {
            id: format!("{} {}", galaxy, id).to_string(),
            galaxy: galaxy.to_string(),
            level: level.to_string(),
            guards: "?".to_string(),
            boss: "?".to_string(),
        };

        // handle the ships now
        static RE_SHIPS: Lazy<Regex> = Lazy::new(|| Regex::new(
            r"DX[0-9]{1,5}\u0000.*?(?<ship>[[:word:]\. ]*)\u{0000}(Light Fighter|Heavy Fighter|Support Freighter|Capital Ship|Organic)"
        ).unwrap());
        
        let caps_ship = RE_SHIPS.captures_iter(a);
        // 4 cases:
        // 1: matches the boss -> must be a guard, as only 1 boss ship -> SWAP boss and guard
        // 2: matches the guard -> guard, do nothing
        // 3: matches neither -> first scanned item, put as a guard
        /*
        boss, guard, guard, guard
            1. boss  :: != data.boss, != data.guard :: boss -> data.guard
            2. guard :: != data.boss, != data.guard :: data.guard -> data.boss :: guard -> data.guard
            3. guard :: == data.guard
        */
        let (_, [ship, _]) = caps_ship.next().unwrap().extract();
        data.guard = ship;

        for cap in caps_ship {
            let (_, [ship, _]) = cap.extract();

            if (ship == data.guard) {
                // pass
            } else { // ship does not match existing guard
                if (data.boss == "?") {
                    data.boss = ship;
                } else if (ship != data.boss) {
                    panic!("Ship does not match either boss or guard");
                } else {
                    data.boss = data.guard;
                    data.guard = ship;
                }
            }
        }

        (galaxy, data)
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
    if metadata(raw_path).is_ok() {
        let file = File::open(raw_path).unwrap();
        let reader = BufReader::new(file);

        let mut dg_data: HashMap<String, DgLevel> = serde_json::from_reader(reader).unwrap();
    } else {
        let dg_data: HashMap<String, DgLevel> = HashMap::new();
    }

    // let mut curr_dg: String = "".to_string();
    let mut dg_meta_packet: String = "".to_owned();

    while running.load(Ordering::SeqCst) {
        let packet = cap.next_packet().unwrap();
        let data = String::from_utf8_lossy(packet.data);

        static RE_META: Lazy<Regex> = Lazy::new(|| Regex::new(r"[\x00-\x1F]DG ").unwrap());
        if RE_META.is_match(&data) {
            // curr_dg = parse_dg(&data);
            dg_meta_packet.push_str(&data);
        }
        if dg_meta_packet != "" {
            dg_meta_packet.push_str(&data);
        }
        if data.contains(format!("Entering DG {}", curr_dg).as_str()) {
            // TODO might need to update this to include data packet up to the Entering DG part
            let (gal, dg_level_data) = DgLevel::new(&dg_meta_packet);
            let _ = dg_data.insert(gal, dg_level_data);
        }
    }

    let file = File::open(raw_path).unwrap();
    let writer = BufWriter::new(file);

    let _ = serde_json::to_writer(writer, dg_data);
}
