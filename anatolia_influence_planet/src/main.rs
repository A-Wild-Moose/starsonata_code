use std::sync::{Arc, atomic::{AtomicBool, Ordering}};
use std::fs::File;
use std::io::{BufWriter, Write};//, BufReader, Read};

use notify_rust::{Notification, Timeout};
use regex::Regex;
use once_cell::sync::Lazy;

use anatolia_influence_planet::device::get_pcap_capture;

#[derive(Debug)]
pub struct GalaxyPacket {
    pub packet: String,
    pub data: String,
    galaxy: String
}

impl GalaxyPacket {
    pub fn new() -> Self {
        Self {
            packet: "".to_string(),
            data: "".to_string(),
            galaxy: "".to_string()
        }
    }

    pub fn accumulate(&mut self, a: &str) {
        // static RE_START: Lazy<Regex> = Lazy::new(|| Regex::new(r"[\x00-\x1F]DG ").unwrap());
        static RE_END: Lazy<Regex> = Lazy::new(|| Regex::new(r"Entering (?<gal>[[:word:]' ]*)\.[\x00-\x1F]").expect("Unable to create entering regex"));

        if let Some(m) = RE_END.captures(a) {
            let gal_match = m.name("gal").expect("Unable to parse galaxy name");
            self.packet.push_str(&a[0..gal_match.start()]);
            self.galaxy = gal_match.as_str().to_string();
            println!("{}", self.galaxy);
            // trim to the start of the galaxy information now
            let re_start = Regex::new(self.galaxy.as_str()).expect("Unable to create start regex");
            let last_match = re_start.find_iter(&self.packet).last().expect("Unable to find last iter on galaxy name");
            self.data = self.packet[last_match.end()..].to_string();
        } else {
            self.packet.push_str(a);
        }
    }

    pub fn reset(&mut self) {
        self.packet = "".to_string();
        self.galaxy = "".to_string();
    }
    pub fn reset_data(&mut self) {
        self.data = "".to_string();
    }
}

// fn main() {
//     let mut data = String::new();
//     let f = File::open("raw/out_example.txt").expect("unable to read");
//     let mut f = BufReader::new(f);
//     f.read_to_string(&mut data).expect("should be able to read");

//     let mut gp = GalaxyPacket::new();
//     gp.accumulate(data.as_str());

//     println!("{}", gp.packet);

//     if gp.packet.contains("Influence Collector DropOffPoint"){
//         Notification::new()
//             .appname("Star Sonata")
//             .summary("Anatolia Influence")
//             .body("Influence Turn in")
//             .icon("firefox")
//             .timeout(Timeout::Milliseconds(6000)) //milliseconds
//             .show()
//             .expect("no notification");
//     }
// }

fn main() {
    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();

    ctrlc::set_handler(move || {
        r.store(false, Ordering::SeqCst);
    }).expect("Error setting Ctrl-C handler");

    let mut cap = get_pcap_capture();

    let raw_path = std::path::Path::new("raw/out.txt");
    std::fs::create_dir_all(raw_path.parent().unwrap()).unwrap();
    let f = File::create(raw_path).expect("Unable to create output log file");
    let mut f = BufWriter::new(f);

    let mut gp = GalaxyPacket::new();

    while running.load(Ordering::SeqCst) {
        let Ok(packet) = cap.next_packet() else {
            continue;
        };
        let data = String::from_utf8_lossy(packet.data);

        f.write_all("Start Packet:\n".as_bytes()).expect("unable to write packet header to file");
        f.write_all(data.as_bytes()).expect("unable to write data packet to file");
        f.write_all("\nEnd Packet\n\n\n".as_bytes()).expect("Unable to write packet tail to file");

        gp.accumulate(&data);

        if gp.data.contains("Influence Collector DropOffPoint"){
            Notification::new()
                .appname("Star Sonata")
                .summary("Anatolia Influence")
                .body("Influence Turn in")
                .icon("firefox")
                .timeout(Timeout::Milliseconds(6000)) //milliseconds
                .show()
                .expect("Unable to show notification");
        }
        gp.reset_data();
    }
}
