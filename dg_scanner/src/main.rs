use std::sync::{Arc, atomic::{AtomicBool, Ordering}};
use pcap::{Device, Capture};

use std::fs::File;
use std::io::Write;

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

    let mut f = File::create("packet_dump.log").expect("unable to create file");

    while running.load(Ordering::SeqCst) {
        let packet = cap.next_packet().unwrap();
        let data = String::from_utf8_lossy(packet.data);
        f.write_all(data.as_bytes()).expect("unable to write to file");
    }
}
