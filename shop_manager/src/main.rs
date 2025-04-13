use std::sync::{Arc, atomic::{AtomicBool, Ordering}};
use std::fs::File;
use std::io::{BufWriter, Write, BufReader, Read};
use pcap::{Device, Capture};
use regex::{Regex, Captures, CaptureMatches};
use once_cell::sync::Lazy;

fn main() {
    let file = File::open("raw/raw_part.txt").unwrap();
    let mut rdr = BufReader::new(file);

    let mut buf: Vec<u8> = vec![];

    let _ = rdr.read_to_end(&mut buf).unwrap();
    let data = String::from_utf8_lossy(&buf);

    static RE_MC: Lazy<Regex> = Lazy::new(|| Regex::new(
        r"\x10\x09(?<item>[[:word:] '-]*)[\r\n]{2}   Average(?ms:.*?)Most profitable"
    ).unwrap());

    // [ 0]  12.17b (   1): Free Market: Cult of Labrador
    static RE_shops: Lazy<Regex> = Lazy::new(|| Regex::new(
        r"  \[..\][ ]*(?<price>[\d\.]*[tbm]?) \(.*\): .*: (?<shop>.*)[\r\n]"
    ).unwrap());

    for cmatch in RE_MC.find_iter(&data) {
        let mc_data = cmatch.as_str();

        let cap = RE_MC.captures(cmatch.as_str()).unwrap();
        let item = cap.name("item").unwrap().as_str();

        println!("{}, {}, {}", cmatch.start(), cmatch.end(), item);
        
        for shop_caps in RE_shops.captures_iter(mc_data) {
            let (_, [price, shop]) = shop_caps.extract();
            println!("\t{}, {}", price, shop);
        }
    }

    // for cap in RE_MC.captures_iter(&data) {
    //     let (_, [item]) = cap.extract();
    //     let cmatch = cap.get(1).unwrap();

    //     println!("{}, {}, {}", cmatch.start(), cmatch.end(), item);

    //     for shop_caps in RE_shops.captures_iter(&data[cmatch.start()..cmatch.end()]) {
    //         let (_, [price, shop]) = shop_caps.extract();
    //         println!("\t{}, {}", price, shop);
    //     }
    // }
}

fn main1() {

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

    let file = File::create("raw/raw.txt").unwrap();
    let mut wrt = BufWriter::new(&file);

    while running.load(Ordering::SeqCst) {
        let packet = cap.next_packet().unwrap();
        let data = String::from_utf8_lossy(packet.data);
        
        wrt.write(&packet.data).unwrap();
    }
}
