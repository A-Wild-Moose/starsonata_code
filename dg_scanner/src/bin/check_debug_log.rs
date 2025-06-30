use std::fs::File;
use std::io::{BufReader, Read};

use dg_scanner::dg_data_sql::{DgPacket, DgLevel};

fn main() {
    let file = File::open("raw/debug_gs.log").unwrap();
    let mut rdr = BufReader::new(file);

    let mut data = String::new();

    let _ = rdr.read_to_string(&mut data);

    let mut dgp = DgPacket::new();

    for packet in data.split("\n\n") {
        dgp.accumulate(&packet);
        if dgp.complete {
            let _dg_level_data = DgLevel::new(&dgp);
            dgp.reset();
        }
    }
}