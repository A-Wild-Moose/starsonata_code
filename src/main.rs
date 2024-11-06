// use std::net::{IpAddr, Ipv4Addr};
use pcap::{Device, Capture};
use regex::{Regex, Captures, CaptureMatches};
use once_cell::sync::Lazy;

// liberty_starsonata_com = IpAddr::V4(Ipv4Addr::new(51, 222, 248, 34));

struct ScanData {
    // Metadata
    orbitee: Vec<String>;
    orbiter: Vec<String>;
    gravity: Vec<String>;
    temp: Vec<String>;
    climate: Vec<String>;
    slots: Vec<i64>;
    // Extractor slots, ruins, etc
    item: Vec<String>;
    count: Vec<i64>;
}


impl ScanData {
    fn new(&mut self) {
        self.orbitee = Vec::new();
        self.orbiter = Vec::new();
        self.gravity = Vec::new();
        self.temp = Vec::new();
        self.climate = Vec::new();
        self.slots = Vec::new();
        self.item = Vec::new();
        self.count = Vec::new();
    }

    fn add_scan(&mut self, cap_meta: Captures, cap_exe: CaptureMatches) {
        let mut n = 0;

        for cap in cap_exe {
            let (_, [item, count]) = cap.extract();
            self.item.push(item.to_string());
            self.count.push(count.parse::<i64>().unwrap());
            n = n + 1;
        }
    }
}


fn parse_entering(a: &str) -> &str {
    let i1 = a.find("Entering").unwrap();
    let mut i2 = i1 + 1;
    if a.contains("Galaxy owned") {
        i2 = i1 + a.get(i1..a.len()).unwrap().find(". Galaxy owned").unwrap();
    } else {
        i2 = i1 + a.get(i1..a.len()).unwrap().find("\0").unwrap() - 1;
    }

    a.get(i1..i2).unwrap()
}

fn parse_scan(a: &str, scan_data: &mut ScanData) {
    let i1 = a.find("Scan: ").unwrap();
    let i2 = i1 + a.get(i1..a.len()).unwrap().find("\0").unwrap();

    let scan = a.get(i1..i2).unwrap();

    static RE_META: Lazy<Regex> = Lazy::new(|| Regex::new(
        r"Scan: \[(?<orbiter>[[:word:] ]*) \((?<orbitee>[[:word:] \(\)]*)\)\] (?<gravity>[[:word:]]*) Gravity, (?<temp>[[:word:]]*), (?<climate>[[:word:]]*). Base Slots: (?<slots>\d)\.|\[\[([[:word:] ]*)?\]\]"
    ).unwrap());

    let caps = RE_META.captures(scan).unwrap();

    // orbiter.push(caps.name("orbiter").map_or("".to_string(), |m| m.as_str().to_string()));

    // println!("{}", &caps["orbiter"]);
    // println!("{}", &caps["orbitee"]);
    // println!("{}", &caps["gravity"]);
    // println!("{}", &caps["temp"]);
    // println!("{}", &caps["climate"]);
    // println!("{}", &caps["slots"]);

    // println!("{}", caps.len());
    // println!("{}", &caps[6]);

    // TODO: need to add ruins
    static RE_EXTRACTORS: Lazy<Regex> = Lazy::new(|| Regex::new(
        r"\[\[(?<link>[\w\s]*)\]\] \((?<ext>\d{1,3})\)"
    ).unwrap());

    for cap in RE_EXTRACTORS.captures_iter(scan) {
        let (_, [c1, c2]) = cap.extract();
        dbg!(c1, c2);
    }
}

fn main() {
    
    // let mut main_device = Device::lookup().unwrap().unwrap();
    // let devices = Device::list().unwrap();

    // println!("testing");

    // // println!("{}", main_device.name);

    // for dev in devices.iter(){
    //     // println!("{}, {}", dev.name, dev.desc.clone().unwrap());
    //     if dev.desc.clone().unwrap().contains("Mullvad Tunnel") {
    //         main_device = dev.clone();
    //     }
    // }


    // let mut cap = Capture::from_device(main_device).unwrap().open().unwrap();
    
    // let _ = cap.filter(
    //     "src host 51.222.248.34", true
    // );

    // let mut i = 0;
    // while i < 50 {
    //     let packet = cap.next_packet().unwrap();
    //     let data = String::from_utf8_lossy(packet.data);
    //     if data.contains("Entering "){
    //         let line = parse_entering(&data);
    //         println!("{}", line);
    //     }
    //     println!("received packet! {:?}", String::from_utf8_lossy(packet.data));
    //     i = i + 1;
    //     // received packet! "E\0\u{3}\u{c}\rT@\02\u{6}��3��\"\n�_j\u{b}����\u{17}�\u{5}�\0��P\u{18}\u{1}���\0\0\u{4}\0\u{c}�1�\u{5} 
    //     // \0r�W�\u{6}�ڙ\0\0\0\0\0\0\0\0\0\0\0\0\0\0��@\0\0\0\0\0@�@ \0r�W�\u{6}�ڙ\0\0\0\0\0\0\0\0\0\0\0\0\0\0��@\0\0\0\0\0@�@�\0\u{10}
    //     // \0Scan: [Arabian Nights (Main Sequence Sun (O2V class))] Heavy Gravity, Blistering, Terran. Base Slots: 4. Detected resources: A bunch of [[Metals]] (10), A bunch of [[Silicon]] (13), A bunch of [[Nuclear Waste]] (14).\0\0�\0o-�\0\0Scan: [Arabian Nights (Main Sequence Sun (O2V class))] Heavy Gravity, Blistering, Terran. Base Slots: 4. Detected resources: A bunch of [[Metals]] (10), A bunch of [[Silicon]] (13), A bunch of [[Nuclear Waste]] (14).\0\0333333�?�2\0\0'\0 \0\0Scanner.wav\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\u{15}\0!�W�\u{6}r(V�\u{5ee}�@\0�D�\u{1b}܃�\0\u{15}\0!�W�\u{6}r(V�\u{5ee}�@\0�D�\u{1b}܃�\0\u{15}\0!�W�\u{6}��C���@\0�D�\u{1b}܃�\0\u{15}\0!�W�\u{6}��C���@\0�D�\u{1b}܃�\0?\0\u{b}���\u{8}�\u{1b}Nf�+���K�Zmә@���`�\u{10}���&��,\r�?��n!C�\u{3}�\nᮉ�\0h@\0\0\0\0\0\0\0\0\0\0\0"
    // }

    const E1: &str = "E\0\u{3}\u{c}\rT@\02\u{6}��3��\"\n�_j\u{b}����\u{17}Entering Sol. Galaxy owned by Earthforce.\0\0r�W�\"";
    println!("{}", parse_entering(E1));

    const E2: &str = "E\0\u{3}\u{c}\rT@\02\u{6}��3��\"\n�_j\u{b}����\u{17}Entering Sol 4.1298.\0\0r�W�\"";
    println!("{}", parse_entering(E2));

    const E3: &str = "E\0\u{3}\u{c}\rT@\02\u{6}��3��\"\n�_j\u{b}����\u{17}Entering Iq'bana.\0\0r�W�\"";
    println!("{}", parse_entering(E3));

    const DATA: &str = "E\0\u{3}\u{c}\rT@\02\u{6}��3��\"\n�_j\u{b}����\u{17}�\u{5}�\0��P\u{18}\u{1}���\0\0\u{4}\0\u{c}�1�\u{5} \0r�W�\u{6}�ڙ\0\0\0\0\0\0\0\0\0\0\0\0\0\0��@\0\0\0\0\0@�@ \0r�W�\u{6}�ڙ\0\0\0\0\0\0\0\0\0\0\0\0\0\0��@\0\0\0\0\0@�@�\0\u{10}\0Scan: [Arabian Nights (Main Sequence Sun (O2V class))] Heavy Gravity, Blistering, Terran. Base Slots: 4. Detected resources: A bunch of [[Metals]] (10), A bunch of [[Silicon]] (13), A bunch of [[Nuclear Waste]] (14).\0\0�\0o-�";

    let mut orbitee: Vec<String> = Vec::new();
    let mut orbiter: Vec<String> = Vec::new();
    let mut gravity: Vec<String> = Vec::new();
    let mut temp: Vec<String> = Vec::new();
    let mut climate: Vec<String> = Vec::new();
    let mut slots: Vec<I64> = Vec::new();

    println!("{}", &caps["orbitee"]);
    println!("{}", &caps["gravity"]);
    println!("{}", &caps["temp"]);
    println!("{}", &caps["climate"]);
    println!("{}", &caps["slots"]);
    let res = parse_scan(DATA, &mut orbitee);
}
