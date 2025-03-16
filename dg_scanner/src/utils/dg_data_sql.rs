use std::path::Path;
use std::fs::metadata;

use once_cell::sync::Lazy;
use regex::Regex;
use rusqlite::Connection;

#[derive(Debug)]
struct DgLevel {
    name: String,  // galaxy + level
    id: String,  // decimal ID
    galaxy: String,  // galaxy name
    level: String,  // full level name
    guard: String,
    boss: String,
}

impl DgLevel {
    pub fn new(a: &str) -> Self {
        static RE_DG: Lazy<Regex> = Lazy::new(|| Regex::new(
            r"DG (?<dg_gal>[[:word:]' ]*) (?<dg_level>[0-9]{1,2}\.[0-9]+[A-Z]?)"
        ).unwrap());
        let caps = RE_DG.captures(a).expect("Unable to parse the DG name");

        // this SHOULD be the first match, which should be the level we are actually entering
        let galaxy = caps.name("dg_gal").unwrap().as_str();
        let level = caps.name("dg_level").unwrap().as_str();
        let (_, id) = level.split_once(".").unwrap();  // can contain A/B/C/D split ids

        println!("New DG Level - galaxy: {} level {} id: {}", galaxy, level, id);

        // allocate the data
        let mut data = Self {
            name: format!("{} {}", galaxy, level).to_string(),
            id: id.to_string(),
            galaxy: galaxy.to_string(),
            level: level.to_string(),
            guard: "?".to_string(),
            boss: "".to_string(),
        };

        // handle the ships now
        static RE_SHIPS: Lazy<Regex> = Lazy::new(|| Regex::new(
            r"DX[0-9]{1,5}\u0000(?s:.*?)[\x00-\x1F](?<ship>[[:word:]'\. ]*)\u0000(Light Fighter|Heavy Fighter|Support Freighter|Capital Ship|Organic)"
        ).unwrap());

        let mut caps_ship = RE_SHIPS.captures_iter(a);

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
            } else { // ship does not match existing guard
                if data.boss == "" {
                    data.boss = ship.to_string();
                } else if ship != data.boss {
                    panic!("Ship does not match either boss or guard");
                } else {
                    data.boss = data.guard;
                    data.guard = ship.to_string();
                }
            }
        }

        if data.boss == "" {
            println!("\t{}", data.guard);
        } else {
            println!("\t{}, {}", data.guard, data.boss);
        }

        // return data
        data
    }

    pub fn add_to_database(&self, db_conn: &Connection) {
        let _ = db_conn.execute(
            "INSERT OR REPLACE INTO DgData (name, id, galaxy, level, guard, boss) 
            SELECT ?1, ?2, ?3, ?4, ?5, ?6
            WHERE NOT EXISTS (SELECT * FROM DgData WHERE name=?1 AND guard<>'?')",
            (&self.name, &self.id, &self.galaxy, &self.level, &self.guard, &self.boss)
        ).unwrap();
        // let _ = db_conn.execute(
        //     "INSERT OR REPLACE INTO DgData (name, id, galaxy, level, guard, boss)
        //     VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        //     (&self.name, &self.id, &self.galaxy, &self.level, &self.guard, &self.boss)
        // ).unwrap();
    }
}

// DG DATA HANDLING
pub struct DgData<'a> {
    db: Connection,
    raw_path: &'a Path
}

impl<'a> DgData<'a> {
    pub fn new(path: &'a str) -> DgData<'a> {
        if metadata(path).is_ok() {
            Self{
                db: Connection::open(path).unwrap(),
                raw_path: Path::new(path)
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
                    galaxy VARCHAR(40),
                    level VARCHAR(10),
                    guard VARCHAR(40),
                    boss VARCHAR(40)
                )",
                (),   // no parameters for table creation
            ).unwrap();

            Self{
                db: conn,
                raw_path: path
            }
        }
    }

    pub fn update(&self, dg_multi_packet: &str) {
        let dg_level_data = DgLevel::new(dg_multi_packet);
        dg_level_data.add_to_database(&self.db);
    }

    pub fn print_db(&self) {
        let mut stmt = self.db.prepare("SELECT name, id, guard FROM DgData").unwrap();
        let dg_iter = stmt.query_map([], |row| {
            Ok(DgLevel{
                name: row.get(0)?,
                id: row.get(1)?,
                galaxy: "".to_string(),
                level: "".to_string(),
                guard: row.get(2)?,
                boss: "".to_string()
            })
        }).unwrap();

        for dg in dg_iter {
            println!("dg: {:?}", dg.unwrap());
        }
    }
}