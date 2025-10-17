use std::sync::{Arc, atomic::{AtomicBool, Ordering}};
use regex::Regex;
use once_cell::sync::Lazy;
// use colored::Colorize;
// use thousands::Separable;

use std::fs::File;
use std::io::{BufWriter, Write};

use NF_tracker::device::get_pcap_capture;


fn main() {
    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();

    ctrlc::set_handler(move || {
        r.store(false, Ordering::SeqCst);
    }).expect("Error setting Ctrl-C handler");

    let mut cap = match get_pcap_capture() {
        Ok(a) => a,
        Err(e) => panic!("{}", e)
    };

    static RE_roll: Lazy<Regex> = Lazy::new(|| Regex::new(
        r"Neuro Roll \d - new mods: \[(?<mods>[\w\|+ ]*)\]"
    ).unwrap());


    let f = File::create("raw/rolls.log").expect("Should be able to create file");
    let mut f = BufWriter::new(f);
    
    println!("before while");
    while running.load(Ordering::SeqCst) {
        let Ok(packet) = cap.next_packet() else {
            continue;
        };
        let data = String::from_utf8_lossy(packet.data);

        for roll_caps in RE_roll.captures_iter(&data) {
            let (_, [mods]) = roll_caps.extract();
            println!("{}", mods);
            f.write_all(mods.as_bytes()).expect("Should be able to write data");
            f.write_all("\n".as_bytes()).expect("Should be able to write data");
        }

        // f.write_all(data.as_bytes()).expect("Should be able to write data");
        // f.write_all("\n\n".as_bytes()).expect("Should be able to write data");

    }
}