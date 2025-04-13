use std::fs::File;
use std::io::{BufReader, Read};

use once_cell::sync::Lazy;
use regex::Regex;

fn main() {
    let file = File::open("raw/debug_gs.log").unwrap();
    let mut rdr = BufReader::new(file);

    let mut data = String::new();

    let _ = rdr.read_to_string(&mut data);

    // handle the ships now
    static RE_SHIPS: Lazy<Regex> = Lazy::new(|| Regex::new(
        r"DX[0-9]{1,5}\u0000(?s:.*?)[\x00-\x1F](?<ship>[[:word:]'\. ]*)\u0000(Light Fighter|Heavy Fighter|Support Freighter|Capital Ship|Organic)"
    ).unwrap());

    let mut caps_ship = RE_SHIPS.captures_iter(&data);

    // get first ship, return empty if there are no AI
    if let Some(c) = caps_ship.next() {
        let (_, [ship, _]) = c.extract();
        println!("{}", ship);
    } else {
        println!("No ships found");
    }

    // iterate over the remaining ships as possible
    for cap in caps_ship {
        let (_, [ship, _]) = cap.extract();

        println!("{}", ship);
    }
}