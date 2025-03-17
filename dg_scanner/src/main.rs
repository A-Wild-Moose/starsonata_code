use std::sync::{Arc, atomic::{AtomicBool, Ordering}};
use std::fs::File;
use std::io::{BufWriter, Write};

use utils::device::get_pcap_capture;
// use utils::dg_data::{DgLevel, DgData};
use utils::dg_data_sql::{DgPacket, DgData};
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
    // let mut dg_meta_packet: String = "".to_owned();
    let mut dg_meta_packet = DgPacket::new();

    while running.load(Ordering::SeqCst) {
        let Ok(packet) = cap.next_packet() else {
            continue;
        };
        let data = String::from_utf8_lossy(packet.data);

        // handle adding this to a larger packet of data about the DG level
        dg_meta_packet.accumulate(&data);
        if dg_meta_packet.complete {
            write!(f, "{}\n\n", dg_meta_packet.packet).expect("Unable to write debug log");
            // update the dg data store
            dg_data.update(&dg_meta_packet.packet);
            // reset the meta packate to empty for the next dg level
            dg_meta_packet.reset();
        }
    }
}
