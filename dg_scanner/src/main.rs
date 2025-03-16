use std::sync::{Arc, atomic::{AtomicBool, Ordering}};
use std::fs::File;
use std::io::{BufWriter, Write};
use regex::Regex;
use once_cell::sync::Lazy;

use utils::device::get_pcap_capture;
// use utils::dg_data::{DgLevel, DgData};
use utils::dg_data_sql::DgData;
mod utils;



fn main() {
    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();

    ctrlc::set_handler(move || {
        r.store(false, Ordering::SeqCst);
    }).expect("Error setting Ctrl-C handler");

    let mut cap = get_pcap_capture();

    // dg data store, with the save file
    // let mut dg_data = DgData::new("raw/raw_dgs_kd.json");
    let dg_data = DgData::new("raw/dgs.db3");

    // debug log
    let debug_path = std::path::Path::new("raw/debug.log");
    std::fs::create_dir_all(debug_path.parent().unwrap()).unwrap();
    let f = File::create(debug_path).expect("Unable to create debug log file");
    let mut f = BufWriter::new(f);

    // let mut curr_dg: String = "".to_string();
    let mut dg_meta_packet: String = "".to_owned();

    while running.load(Ordering::SeqCst) {
        let Ok(packet) = cap.next_packet() else {
            continue;
        };
        let data = String::from_utf8_lossy(packet.data);

        static RE_META: Lazy<Regex> = Lazy::new(|| Regex::new(r"[\x00-\x1F]DG ").unwrap());
        if let Some(m) = RE_META.find(&data) {
            dg_meta_packet.push_str(&data[m.start()..]);
        }
        if dg_meta_packet != "" {
            dg_meta_packet.push_str(&data);
        }
        if let Some(i1) = data.find("Entering DG ") {
            dg_meta_packet.push_str(&data[0..i1]);

            write!(f, "{}\n\n", dg_meta_packet).expect("Unable to write debug log");

            // update the dg data store
            dg_data.update(&dg_meta_packet);

            // reset the meta packet to empty for the next dg level
            dg_meta_packet = "".to_string();
        }
    }
}
