use serde::{Deserialize, Serialize};
use once_cell::sync::Lazy;
use regex::Regex;


#[derive(Serialize, Deserialize)]
pub struct DgLevel {
    id: String,
    galaxy: String,
    level: String,
    pub guards: String,
    boss: String
}

impl DgLevel {
    pub fn new(a: &str) -> (String, Self) {
        static RE_DG: Lazy<Regex> = Lazy::new(|| Regex::new(
            r"DG (?<dg_gal>[[:word:] ]*) (?<dg_level>[0-9]{1,2}\.[0-9]+[A-Z]?)"
        ).unwrap());
        let caps = RE_DG.captures(a).expect("Unable to parse the DG name");

        // this SHOULD be the first match, which should be the level we are actually entering
        let galaxy = caps.name("dg_gal").unwrap().as_str();
        let level = caps.name("dg_level").unwrap().as_str();
        let (_, mut id) = level.split_once(".").unwrap();

        // Remove any characters
        id = id.trim_end_matches(&['A', 'B', 'C', 'D']);

        println!("New DG Level - galaxy: {} level: {} id: {}", galaxy, level, id);

        let mut data = Self {
            id: format!("{} {}", galaxy, id).to_string(),
            galaxy: galaxy.to_string(),
            level: level.to_string(),
            guards: "?".to_string(),
            boss: "?".to_string(),
        };

        // handle the ships now
        static RE_SHIPS: Lazy<Regex> = Lazy::new(|| Regex::new(
            r"DX[0-9]{1,5}\u0000(?s:.*?)[\x00-\x1F](?<ship>[[:word:]'\. ]*)\u0000(Light Fighter|Heavy Fighter|Support Freighter|Capital Ship|Organic)"
        ).unwrap());
        
        let mut caps_ship = RE_SHIPS.captures_iter(a);

        // get first ship, return empty if there are no AI
        if let Some(c) = caps_ship.next() {
            let (_, [ship, _]) = c.extract();
            data.guards = ship.to_string();
        } else {
            return (format!("{} {}", galaxy, level), data) 
        }

        // iterate over the remaining ships as possible
        for cap in caps_ship {
            let (_, [ship, _]) = cap.extract();

            if ship == data.guards {
                // pass
            } else { // ship does not match existing guard
                if data.boss == "?" {
                    data.boss = ship.to_string();
                } else if ship != data.boss {
                    panic!("Ship does not match either boss or guard");
                } else {
                    data.boss = data.guards;
                    data.guards = ship.to_string();
                }
            }
        }

        if data.boss == "?" {
            println!("\t{}", data.guards);
        } else {
            println!("\t{}, {}", data.guards, data.boss);
        }

        (format!("{} {}", galaxy, level), data)
    }
}