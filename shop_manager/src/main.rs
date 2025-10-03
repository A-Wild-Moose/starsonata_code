use std::sync::{Arc, atomic::{AtomicBool, Ordering}};
use regex::Regex;
use once_cell::sync::Lazy;
use colored::Colorize;
use thousands::Separable;

use shop_manager::device::get_pcap_capture;


fn convert_price(a: &str) -> i64 {
    let b = a.replace(",", "");  // american style, know that these are thousands separators

    if b.ends_with(&['t', 'b','m'][..]) {
        let (v, suf) = b.split_at_checked(b.len() - 1).unwrap();
        let mut val = v.parse::<f64>().unwrap();
        match suf {
            "t" => val = val * f64::powi(10.0, 12),
            "b" => val = val * f64::powi(10.0, 9),
            "m" => val = val * f64::powi(10.0, 6),
            _ => panic!("unexpected suffix")
        }
        val as i64
    } else {
        a.parse::<i64>().unwrap()
    }
}

fn check_update_price(prices: Vec<i64>, shops: Vec<String>) -> (Option<i64>, Option<i64>, Option<i64>){
    let original_price = match shops.iter().position(|r| r == "Shadow Shop") {
        Some(a) => Some(prices[a]),
        None => None
    };

    let (cheapest_price, new_price) = if prices.len() > 0 {
        let np = match shops[0].as_str() {
            "Shadow Shop" => prices[0],
            "Cartel Shop" => prices[0],
            "KMART" => prices[0],
            "YP Shop" => prices[0],
            "Slade's Shop" => prices[0],
            "zel fine dining" => prices[0],
            "MBGA Shop" => prices[0],
            _ => (0.99 * prices[0] as f64) as i64
        };
        (Some(prices[0]), Some(np))
    } else {
        (None, None)
    };
    (original_price, cheapest_price, new_price)
}

fn print_price_update(item: &str, original: Option<i64>, cheapest: Option<i64>, new: Option<i64>) -> Option<String> {
    let width = 40;
    // case 1: original == cheapest == new
    // case 2: original > cheapest > new
    // case 3: None, cheapest > new
    // case 4: none available
    // sellprice | buyprice | maxbuy | maxsell | maxmake | name
    let (msg, line) = match (original, cheapest, new) {
        (Some(o), Some(c), Some(n)) => {
            match o > c {
                true => (
                    format!(
                        "{:width$} :: original: {:15} cheapest: {:15} now selling: {:15}",
                        item,
                        o.separate_with_commas().red(),
                        c.separate_with_commas().yellow(),
                        n.separate_with_commas().green()
                    ),
                    Some(format!("{n} | 1 | 100 | 0 | 0 | {item}").to_string())
                ),
                false => (
                    format!(
                        "{:width$} :: original: {:15} cheapest: {:15} now selling: {:15}",
                        item,
                        o.separate_with_commas().green(),
                        c.separate_with_commas().green(),
                        n.separate_with_commas().green()
                    ),
                    Some(format!("{n} | 1 | 100 | 0 | 0 | {item}").to_string())
                )
            }
        },
        (None, Some(c), Some(n)) => (
            format!(
                "{:width$} :: original: {:15} cheapest: {:15} now selling: {:15}",
                item,
                "-".red(),
                c.separate_with_commas().yellow(),
                n.separate_with_commas().green()
            ),
            Some(format!("{n} | 1 | 100 | 0 | 0 | {item}").to_string())
        ),
        (None, None, None) => (
            format!("{:width$} :: {}", item, "no available sale points, manual entry required".red()),
            None
        ),
        _ => panic!("unknown combination of original, cheapest, and new prices.")
    };
    println!("{}", msg);
    line
}


fn main() {
    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();

    ctrlc::set_handler(move || {
        r.store(false, Ordering::SeqCst);
    }).expect("Error setting Ctrl-C handler");

    let mut cap = get_pcap_capture();

    static RE_MC: Lazy<Regex> = Lazy::new(|| Regex::new(
        r"\x10\x09(?<item>[[:word:] '-]*)[\r\n]{2}   Average(?ms:.*?)Most profitable"
    ).unwrap());

    // [ 0]  12.17b (   1): Free Market: Cult of Labrador
    static RE_SHOPS: Lazy<Regex> = Lazy::new(|| Regex::new(
        r"  \[..\][ ]*(?<price>[\d\.]*[tbm]?) \(.*\): .*: (?<shop>.*)[\r\n]"
    ).unwrap());

    let mut lines: Vec<String> = Vec::with_capacity(50);


    while running.load(Ordering::SeqCst) {
        let Ok(packet) = cap.next_packet() else {
            continue;
        };
        let data = String::from_utf8_lossy(packet.data);

        for cmatch in RE_MC.find_iter(&data) {
            let mc_data = cmatch.as_str();

            let cap = RE_MC.captures(cmatch.as_str()).unwrap();
            let item = cap.name("item").unwrap().as_str();

            let mut prices: Vec<i64> = Vec::with_capacity(5);
            let mut shops: Vec<String> = Vec::with_capacity(5);

            for shop_caps in RE_SHOPS.captures_iter(mc_data) {
                let (_, [price, shop]) = shop_caps.extract();
                prices.push(convert_price(price));
                shops.push(shop.to_string());
            }
    
            let (original, cheap, new_price) = check_update_price(prices, shops);
    
            let line = print_price_update(item, original, cheap, new_price);
            match line {
                Some(l) => lines.push(l),
                None => {}
            }
        }
    }

    println!("\n\n{}", "sellprice | buyprice | maxbuy | maxsell | maxmake | name");
    for line in lines.iter() {
        println!("{}", line);
    }
}


// fn main2() {
//     let file = File::open("raw/raw.txt").unwrap();
//     let mut rdr = BufReader::new(file);

//     let mut buf: Vec<u8> = vec![];

//     let _ = rdr.read_to_end(&mut buf).unwrap();
//     let data = String::from_utf8_lossy(&buf);

//     static RE_MC: Lazy<Regex> = Lazy::new(|| Regex::new(
//         r"\x10\x09(?<item>[[:word:] '-]*)[\r\n]{2}   Average(?ms:.*?)Most profitable"
//     ).unwrap());

//     // [ 0]  12.17b (   1): Free Market: Cult of Labrador
//     static RE_shops: Lazy<Regex> = Lazy::new(|| Regex::new(
//         r"  \[..\][ ]*(?<price>[\d\.]*[tbm]?) \(.*\): .*: (?<shop>.*)[\r\n]"
//     ).unwrap());

//     let mut lines: Vec<String> = Vec::with_capacity(50);

//     for cmatch in RE_MC.find_iter(&data) {
//         let mc_data = cmatch.as_str();

//         let cap = RE_MC.captures(cmatch.as_str()).unwrap();
//         let item = cap.name("item").unwrap().as_str();

//         let mut prices: Vec<i64> = Vec::with_capacity(5);
//         let mut shops: Vec<String> = Vec::with_capacity(5);
        
//         for shop_caps in RE_shops.captures_iter(mc_data) {
//             let (_, [price, shop]) = shop_caps.extract();
//             prices.push(convert_price(price));
//             shops.push(shop.to_string());
//         }

//         let (original, cheap, new_price) = check_update_price(prices, shops);

//         let line = print_price_update(item, original, cheap, new_price);
//         match line {
//             Some(l) => lines.push(l),
//             None => {}
//         }
//     }

//     println!("\n\n{}", "sellprice | buyprice | maxbuy | maxsell | maxmake | name");
//     for line in lines.iter() {
//         println!("{}", line);
//     }
// }

// fn main1() {

//     let running = Arc::new(AtomicBool::new(true));
//     let r = running.clone();

//     ctrlc::set_handler(move || {
//         r.store(false, Ordering::SeqCst);
//     }).expect("Error setting Ctrl-C handler");

    
//     let mut main_device = Device::lookup().unwrap().unwrap();
//     let devices = Device::list().unwrap();

//     // println!("{}", main_device.name);

//     for dev in devices.iter(){
//         // println!("{}, {}", dev.name, dev.desc.clone().unwrap());
//         if dev.desc.clone().unwrap().contains("Mullvad Tunnel") {
//             main_device = dev.clone();
//         }
//     }

//     // capture device
//     let mut cap = Capture::from_device(main_device).unwrap().open().unwrap();
    
//     // set the filters on the packat reading
//     let _ = cap.filter(
//         "src host 51.222.248.34", true
//     );

//     let file = File::create("raw/raw.txt").unwrap();
//     let mut wrt = BufWriter::new(&file);

//     while running.load(Ordering::SeqCst) {
//         let packet = cap.next_packet().unwrap();
//         let data = String::from_utf8_lossy(packet.data);
        
//         wrt.write(&packet.data).unwrap();
//     }
// }
