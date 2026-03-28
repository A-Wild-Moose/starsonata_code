use std::sync::{Arc, Mutex, LazyLock};

use regex::{Regex, RegexSet};
use tokio::sync::{mpsc::Sender, Notify};
use tracing::{instrument, info, warn};

use super::device::get_pcap_capture;


// macro for extrating information from a capture.  Used so that we dont panic using the .extract method.
macro_rules! extract_capture {
    ($cap:expr, [$($x:expr),*]) => {
        {
            [$(
                match $cap.name($x) {
                    Some(a) => a.as_str().to_string(),
                    None => "NONE".to_string()
                }
            ),*]
        }
    }
}


// define some macros so that all our colors stay constant
macro_rules! player {
    ($player_string:ident) => {
        format_args!("\u{001b}[0;34m{}\u{001b}[0;0m", $player_string)
    }
}

macro_rules! quant {
    ($quant_string:ident) => {
        format_args!("\u{001b}[0;36m{}\u{001b}[0;0m", $quant_string)
    }
}

macro_rules! item {
    ($item_string:ident) => {
        format_args!("\u{001b}[0;33m{}\u{001b}[0;0m", $item_string)
    }
}

macro_rules! equip_dir {
    ($dir_string:ident) => {
        format_args!("\u{001b}[0;35m{}\u{001b}[0;0m", $dir_string)
    }
}

struct StationMonitor {
    re_set: RegexSet,
    transfer: Regex,
    use_bp: Regex,
    construct: Regex,
    construct_done: Regex,
    equip: Regex,
}

impl StationMonitor {
    fn new() -> Self {
        // +Shadow Wolf transferred 1 Fallen Secondary Desolation Beam* out of base
        // +Shadow Wolf transferred 1 Empyreal Incinerator* out of base
        // +Shadow Wolf transferred 1 Incineration Rocket* out of base

        // +Shadow Wolf transferred 50,000,000 credits to base
        // +Shadow Wolf took 50,000,000 credits from base
        let patterns = vec![
            r"\+(?<player>[[:word:] '\-_]*) (?:transferred|took) (?<quant>[[0-9],]+) (?<item>[[:word:] ,'&\.\-\*]*) (?<dir>to|out of|from) base",
            r"\+(?<player>[[:word:] '\-_]*) using (?<item>[[:word:] '\.\-]*) Blueprint",
            r"\+(?<player>[[:word:] '\-_]*) constructing (?<item>[[:word:] '\.\-]*)",
            r"Construction finished on (?<quant>[0-9]*) (?<item>[[:word:] '\.\-]*)",
            r"(?<player>[[:word:] '\-_]*) (?<dir>(un)?equipped) (?<item>[[:word:] '\-\*]*)x(?<quant>[0-9]+)."
        ];
        Self {
            re_set: RegexSet::new(patterns.clone()).unwrap(),
            transfer: Regex::new(patterns[0]).unwrap(),
            use_bp: Regex::new(patterns[1]).unwrap(),
            construct: Regex::new(patterns[2]).unwrap(),
            construct_done: Regex::new(patterns[3]).unwrap(),
            equip: Regex::new(patterns[4]).unwrap()
        }
    }

    fn get_match_capture(&self, a: &str, tx: Sender<String>) {
        // also handles credits
        for cap in self.transfer.captures_iter(a) {
            let [player, quant, item, dir] = extract_capture!(cap, ["player", "quant", "item", "dir"]);
            let line = format!(
                "{} transferred {} {} {} base",
                player!(player),
                quant!(quant),
                item!(item),
                dir,
            );
            if let Err(why) = tx.blocking_send(line) {
                warn!("Unable to transmit captured line: {:?}", why);
            }
        }
        for cap in self.use_bp.captures_iter(a) {
            let [player, item] = extract_capture!(cap, ["player", "item"]);
            let line = format!(
                "{} using {} Blueprint",
                player!(player),
                item!(item),
            );
            if let Err(why) = tx.blocking_send(line) {
                warn!("Unable to transmit captured line: {:?}", why);
            }
        }
        for cap in self.construct.captures_iter(a) {
            let [player, item] = extract_capture!(cap, ["player", "item"]);
            let line = format!(
                "{} constructing {}",
                player!(player),
                item!(item),
            );
            if let Err(why) = tx.blocking_send(line) {
                warn!("Unable to transmit captured line: {:?}", why);
            }
        }
        for cap in self.construct_done.captures_iter(a) {
            let [quant, item] = extract_capture!(cap, ["quant", "item"]);
            let line = format!(
                "Construction finished on {} {}",
                quant!(quant),
                item!(item),
            );
            if let Err(why) = tx.blocking_send(line) {
                warn!("Unable to transmit captured line: {:?}", why);
            }
        }
        for cap in self.equip.captures_iter(a) {
            let [player, dir, quant, item] = extract_capture!(cap, ["player", "dir", "quant", "item"]);
            let line = format!(
                "{} {} {} {}",
                player!(player),
                equip_dir!(dir),
                quant!(quant),
                item!(item),
            );
            if let Err(why) = tx.blocking_send(line) {
                warn!("Unable to transmit captured line: {:?}", why);
            }
        }
    }
}


#[instrument(skip(tx, cancel_notify))]
pub fn listen_for_prod(tx: Sender<String>, cancel_notify: Arc<Notify>) {
    info!("Getting capture device...");
    let mut cap = get_pcap_capture().unwrap();
    let running = Arc::new(Mutex::new(true));
    let r_clone = running.clone();

    tokio::spawn(async move {
        cancel_notify.notified().await;
        *r_clone.lock().expect("Unable to acquire lock") = false;
    });

    static STATION_MONITOR: LazyLock<StationMonitor> = LazyLock::new(|| StationMonitor::new());

    while *running.lock().expect("Unable to acquire `running` lock.") {
        match cap.next_packet() {
            Ok(packet) => {
                let data = String::from_utf8_lossy(packet.data);

                if STATION_MONITOR.re_set.is_match(&data) {
                    STATION_MONITOR.get_match_capture(&data, tx.clone());
                }
            },
            Err(_) => continue,
        }
    }
}