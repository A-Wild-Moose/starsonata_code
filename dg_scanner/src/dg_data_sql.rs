use std::path::Path;
use std::fs::metadata;

use once_cell::sync::Lazy;
use regex::Regex;
use rusqlite::{Connection, params};


#[derive(Debug)]
pub struct DgPacket {
    pub packet: String,
    in_progress: bool,
    pub complete: bool,
    galaxy: String,
    level: String,
}

impl DgPacket {
    pub fn new() -> Self {
        Self {
            packet: "".to_string(),
            in_progress: false,
            complete: false,
            galaxy: "".to_string(),
            level: "".to_string(),
        }
    }

    pub fn accumulate(&mut self, a: &str) {
        static RE_START: Lazy<Regex> = Lazy::new(|| Regex::new(r"[\x00-\x1F]DG ").unwrap());
        static RE_END: Lazy<Regex> = Lazy::new(|| Regex::new(r"Entering DG (?<dg_gal>[[:word:]' ]*) (?<dg_level>[0-9]{1,2}\.[0-9]+[A-Z]?)\.[\x00-\x1F]").unwrap());

        if let Some(m) = RE_START.find(a) {
            self.reset();
            self.packet.push_str(&a[m.start()..]);
            self.in_progress = true;
        } else if self.in_progress {
            self.packet.push_str(a);
        }

        if let Some(end_cap) = RE_END.captures(a) {
            let level_match = end_cap.name("dg_level").expect("Unable to parse DG level");

            self.packet.push_str(&a[0..level_match.end()]);
            self.in_progress = false;
            self.complete = true;
            self.galaxy = end_cap.name("dg_gal").expect("Unable to parse DG name").as_str().to_string();
            self.level = level_match.as_str().to_string();
        }
    }

    pub fn reset(&mut self) {
        self.packet = "".to_string();
        self.in_progress = false;
        self.complete = false;
        self.galaxy = "".to_string();
        self.level = "".to_string();
    }
}

#[derive(Debug)]
pub struct DgLevel {
    name: String,  // galaxy + level
    id: String,  // decimal ID
    room: i16, // room ID
    galaxy: String,  // galaxy name
    level: String,  // full level name
    guard: String,
    boss: Option<String>,
}

impl DgLevel {
    pub fn new(dg_packet: &DgPacket) -> Self {
        // get the ID for the dg level - post decimal value
        let (room, id) = dg_packet.level.split_once(".").unwrap();  // can contain A/B/C/D split ids

        // allocate the data
        let mut data = Self {
            name: format!("{} {}", dg_packet.galaxy, dg_packet.level).to_string(),
            id: id.to_string(),
            room: room.parse::<i16>().unwrap(),
            galaxy: dg_packet.galaxy.clone(),
            level: dg_packet.level.clone(),
            guard: "?".to_string(),
            boss: None,
        };

        println!("New DG Level - galaxy: {} level {} id: {}", data.galaxy, data.level, id);

        // handle the ships now
        static RE_SHIPS: Lazy<Regex> = Lazy::new(|| Regex::new(
            r"DX[0-9]{1,5}\u0000(?s:.*?)[\x00-\x1F](?<ship>[[:word:]'\. ]*)\u0000(Light Fighter|Heavy Fighter|Support Freighter|Industrial Freighter|Capital Ship|Organic)"
        ).unwrap());

        let mut caps_ship = RE_SHIPS.captures_iter(&dg_packet.packet);

        // get first ship, return empty if there are no AI
        if let Some(c) = caps_ship.next() {
            let (_, [ship, _]) = c.extract();
            data.guard = ship.to_string();
        } else {
            return data
        }

        // iterate over the remaining ships as possible
        for cap in caps_ship {
            let (_, [ship, _]) = cap.extract();

            if ship == data.guard {
                // pass
            } else if data.guard.contains(ship) { // pass for stuff like decrepit phoenix
            } else { // ship does not match existing guard
                if data.boss.is_none() {
                    data.boss = Some(ship.to_string());
                } else if ship != data.boss.clone().unwrap() {
                    panic!("Ship [{}] does not match either boss [{:?}] or guard [{}]", ship, data.boss.clone().unwrap(), data.guard.clone());
                } else {
                    data.boss = Some(data.guard);
                    data.guard = ship.to_string();
                }
            }
        }

        if let Some(ref boss) = data.boss {
            println!("\t{}, {}", data.guard, boss);
        } else {
            println!("\t{}", data.guard);
        }

        // return data
        data
    }

    pub fn add_to_database(&self, db_conn: &Connection) {
        let _ = db_conn.execute(
            "INSERT OR REPLACE INTO DgData (name, id, room, galaxy, level, guard, boss) 
            SELECT ?1, ?2, ?3, ?4, ?5, ?6, ?7
            WHERE NOT EXISTS (SELECT * FROM DgData WHERE name=?1 AND guard<>'?')",
            params![&self.name, &self.id, &self.room, &self.galaxy, &self.level, &self.guard, &self.boss]
        ).unwrap();
        // let _ = db_conn.execute(
        //     "INSERT OR REPLACE INTO DgData (name, id, galaxy, level, guard, boss)
        //     VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        //     (&self.name, &self.id, &self.galaxy, &self.level, &self.guard, &self.boss)
        // ).unwrap();
    }
}

// DG DATA HANDLING
pub struct DgData {
    db: Connection,
}

impl DgData {
    pub fn new(path: &str) -> DgData {
        if metadata(path).is_ok() {
            Self{
                db: Connection::open(path).unwrap(),
            }
        } else {
            let path = Path::new(path);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();

            // setup the table
            let conn = Connection::open(path).unwrap();
            let _ = conn.execute(
                "CREATE TABLE DgData (
                    name VARCHAR(50) PRIMARY KEY,
                    id VARCHAR(5),
                    room SMALLINT(5),
                    galaxy VARCHAR(40),
                    level VARCHAR(10),
                    guard VARCHAR(40),
                    boss VARCHAR(40)
                )",
                (),   // no parameters for table creation
            ).unwrap();

            Self{
                db: conn,
            }
        }
    }

    pub fn update(&self, dg_multi_packet: &DgPacket) {
        let dg_level_data = DgLevel::new(dg_multi_packet);
        dg_level_data.add_to_database(&self.db);
    }
}