// use std::net::{IpAddr, Ipv4Addr};
use std::sync::{Arc, atomic::{AtomicBool, Ordering}};
use pcap::{Device, Capture};
use regex::{Regex, Captures, CaptureMatches};
use once_cell::sync::Lazy;
use polars::{frame::{DataFrame, column::Column}};

// liberty_starsonata_com = IpAddr::V4(Ipv4Addr::new(51, 222, 248, 34));


struct ScanData {
    // Metadata
    galaxy: Vec<String>,
    orbitee: Vec<String>,
    orbiter: Vec<String>,
    gravity: Vec<String>,
    temp: Vec<String>,
    climate: Vec<String>,
    slots: Vec<i64>,
    // Extractor slots, ruins, etc
    item: Vec<String>,
    count: Vec<i64>
}


impl ScanData {
    fn new() -> Self{
        Self {
            galaxy: Vec::new(),
            orbitee: Vec::new(),
            orbiter: Vec::new(),
            gravity: Vec::new(),
            temp: Vec::new(),
            climate: Vec::new(),
            slots: Vec::new(),
            item: Vec::new(),
            count: Vec::new()
        }
    }

    fn add_scan(&mut self, galaxy: &str, cap_meta: Captures, cap_exe: CaptureMatches) {
        let mut n = 0;

        for cap in cap_exe {
            let (_, [item, count]) = cap.extract();
            match item {
                "Ruins" => {
                    self.item.push(format!("Ruins of {}", count).to_string());
                    self.count.push(0);
                }
                _ => {
                    self.item.push(item.to_string());
                    self.count.push(count.replace(",", "").parse::<i64>().unwrap());
                }
            }
            n = n + 1;
        }

        // set all the metadata
        self.galaxy.append(&mut vec![galaxy.to_string(); n]);
        self.orbiter.append(&mut vec![cap_meta.name("orbiter").unwrap().as_str().to_string(); n]);
        self.orbitee.append(&mut vec![cap_meta.name("orbitee").unwrap().as_str().to_string(); n]);
        self.gravity.append(&mut vec![cap_meta.name("gravity").unwrap().as_str().to_string(); n]);
        self.temp.append(&mut vec![cap_meta.name("temp").unwrap().as_str().to_string(); n]);
        self.climate.append(&mut vec![cap_meta.name("climate").unwrap().as_str().to_string(); n]);
        self.slots.append(&mut vec![cap_meta.name("slots").unwrap().as_str().parse::<i64>().unwrap(); n]);

    }
}


fn parse_entering(a: &str) -> String {
    let i1 = a.find("Entering").unwrap();
    
    let i2 = if a.contains("Galaxy owned") {
        i1 + a.get(i1..a.len()).unwrap().find(". Galaxy owned").unwrap()
    } else {
        i1 + a.get(i1..a.len()).unwrap().find("\0").unwrap() - 1
    };

    a.get(i1..i2).unwrap().to_string()
}

fn parse_scan(a: &str, galaxy: &str, scan_data: &mut ScanData) {
    let i1 = a.find("Scan: ").unwrap();
    let i2 = i1 + a.get(i1..a.len()).unwrap().find("\0").unwrap();

    let scan = a.get(i1..i2).unwrap();

    static RE_META: Lazy<Regex> = Lazy::new(|| Regex::new(
        r"Scan: \[(?<orbiter>[[:word:] ']*) \((?<orbitee>[[:word:] '\(\)]*)\)\] (?<gravity>[[:word:]]*) Gravity, (?<temp>[[:word:]]*), (?<climate>[[:word:]]*). Base Slots: (?<slots>\d)\.|\[\[([[:word:] ]*)?\]\]"
    ).unwrap());

    let caps = RE_META.captures(scan).unwrap();

    // TODO: need to add ruins
    static RE_EXTRACTORS: Lazy<Regex> = Lazy::new(|| Regex::new(
        r"\[\[(?<link>[\w\s]*)\]\] \((?<ext>\d{1,3})\)|(Ruins) of (?<ruins>[[:word:] ']*)[\.\,]|(Colony), population: (?<pop>[\d,]*) | of ([[:word:] ']*) \((\d{1,3})\)"
    ).unwrap());

    // update the data store
    scan_data.add_scan(galaxy, caps, RE_EXTRACTORS.captures_iter(scan));
}

fn main() {

    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();

    ctrlc::set_handler(move || {
        r.store(false, Ordering::SeqCst);
    }).expect("Error setting Ctrl-C handler");

    
    let mut main_device = Device::lookup().unwrap().unwrap();
    let devices = Device::list().unwrap();

    // println!("{}", main_device.name);

    for dev in devices.iter(){
        // println!("{}, {}", dev.name, dev.desc.clone().unwrap());
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

    // allocate scan data storage
    let mut scan_data = ScanData::new();

    let mut galaxy = "".to_string();
    while running.load(Ordering::SeqCst) {
        let packet = cap.next_packet().unwrap();
        let data = String::from_utf8_lossy(packet.data);
        if data.contains("Entering "){
            galaxy = parse_entering(&data);
        }
        if data.contains("Scan:"){
            parse_scan(&data, &*galaxy, &mut scan_data);
        }
    }
    
    // create a polars dataframe
    let df = DataFrame::new(
        vec![
            Column::new("Galaxy".into(), scan_data.galaxy),
            Column::new("Solar Body".into(), scan_data.orbiter),
            Column::new("Parent Body".into(), scan_data.orbitee),
            Column::new("Gravity".into(), scan_data.gravity),
            Column::new("Temp".into(), scan_data.temp),
            Column::new("Climate".into(), scan_data.climate),
            Column::new("Base Slots".into(), scan_data.slots),
            Column::new("Resource".into(), scan_data.item),
            Column::new("Extractors/Population".into(), scan_data.count)
        ]
    ).unwrap();

    println!("{:?}", df);
}
