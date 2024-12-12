use std::sync::{Arc, atomic::{AtomicBool, Ordering}};
use std::fs;
use pcap::{Device, Capture};
use regex::{Regex, Captures, CaptureMatches};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use serde_json::Result;

#[derive(Serialize, Deserialize)]
struct DgData {
    id: Vec<String>
    galaxy: Vec<String>
    level: Vec<String>
    boss: Vec<String>
    guards: Vec<String>
    count: Vec<i64>
}

impl DgData {
    fn new() -> Self {
        Self {
            id: Vec::new(),
            galaxy: Vec::new(),
            level: Vec::new(),
            boss: Vec::new(),
            guards: Vec::new(),
            count: Vec::new()
        }
    }

    fn to_polars(&self) -> DataFrame {
        DataFrame::new(
            vec![
                Column::new("id".into(), self.id.clone()),
                Column::new("galaxy".into(), self.galaxy.clone()),
                Column::new("level".into(), self.level.clone()),
                Column::new("boss".into(), self.boss.clone()),
                Column::new("guards".into(), self.guards.clone()),
                Column::new("count".into(), self.count.clone())
            ]
        ).unwrap()
    }

    fn from_polars(&mut self, a: &str) {
        let df = CsvReadOptions::default()
            .with_has_header(true)
            .try_into_reader_with_file_path(Some(a.into()))
            .unwrap()
            .finish()
            .unwrap();
        
        // re-order the columns (reverse order) so that we can pop them
        // this assumes a fixed order (which we should be able to guarantee)
        let df = df.reverse();

        self.id.extend(
            df.column("id").unwrap().str().unwrap().into_no_null_iter()
        );

        // Self {
        //     id: df.pop().unwrap().str().unwrap().into_no_null_iter().collect(),
        //     galaxy: df.pop().unwrap().str().unwrap().into_no_null_iter().collect(),
        //     level: df.pop().unwrap().str().unwrap().into_no_null_iter().collect(),
        //     boss: df.pop().unwrap().str().unwrap().into_no_null_iter().collect(),
        //     guards: df.pop().unwrap().str().unwrap().into_no_null_iter().collect(),
        //     count: df.pop().unwrap().i64().unwrap().into_no_null_iter().collect(),
        // }
    }
}

fn parse_dg(a: &str) -> String {
    static RE_DG: Lazy<Regex> = Lazy::new(|| Regex::new(
        r"DG (?<dg_gal>[[:word:] ]*) (?<dg_level>[0-9]{1,2}\.[0-9]+[A-Z]?)"
    ).unwrap());
    let caps = RE_DG.captures(a).expect("Unable to parse the DG name");

    let dg_gal = caps.name("dg_gal").unwrap().as_str();
    let dg_level = caps.name("dg_level").unwrap().as_str();

    println!("{} {}", dg_gal, dg_level);

    format!("{} {}", dg_gal, dg_level)
}

fn parse_ships(a: &str) {
    static RE_SHIPS: Lazy<Regex> = Lazy::new(|| Regex::new(
        r"DX[0-9]{1,5}\u0000.*?(?<ship>[[:word:]\. ]*)\u{0000}(Light Fighter|Heavy Fighter|Support Freighter|Capital Ship|Organic)"
        // r"(?<ship>[[:word:] ]{5,11})"
    ).unwrap());
    
    let caps_ship = RE_SHIPS.captures_iter(a);
    for cap in caps_ship {
        let (_, [ship, ship_type]) = cap.extract();
        println!("\t{}", ship.to_string());
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
    let raw_path = "raw/raw_dgs_kd.csv";
    if fs::metadata(raw_path).is_ok() {
        let mut dg_data = DgData::new();
        dg_data.from_polars(raw_path);
    } else {
        let dg_data = DgData::new();
    }

    let mut curr_dg: String = "".to_string();

    while running.load(Ordering::SeqCst) {
        let packet = cap.next_packet().unwrap();
        let data = String::from_utf8_lossy(packet.data);

        static RE_META: Lazy<Regex> = Lazy::new(|| Regex::new(r"[\x00-\x1F]DG ").unwrap());
        if RE_META.is_match(&data) {
            curr_dg = parse_dg(&data);
        }
        if curr_dg != "" {
            parse_ships(&data);
        }
        if data.contains(format!("Entering DG {}", curr_dg).as_str()) {
            curr_dg = "".to_string();
        }
    }
}
