use std::sync::{Arc, atomic::{AtomicBool, Ordering}};
use std::fs::File;
use std::io::{BufWriter, Write};
// use once_cell::sync::Lazy;

use planet_scanner::device::get_pcap_capture;

fn main() {
    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();

    ctrlc::set_handler(move || {
        r.store(false, Ordering::SeqCst);
    }).expect("Error setting Ctrl-C handler");

    let mut cap = get_pcap_capture();

    let f = File::create("raw/data.txt").expect("Unable to create file");
    let mut f = BufWriter::new(f);

    while running.load(Ordering::SeqCst) {
        let Ok(packet) = cap.next_packet() else {
            continue;
        };
        let data = String::from_utf8_lossy(packet.data);

        f.write_all(data.as_bytes()).expect("Unable to write data");
    }
}