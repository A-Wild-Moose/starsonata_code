use chrono::prelude::*;

use crate::runs::RunInfo;


impl RunInfo {
    pub fn get_timestamp(&self) -> i64 {
        let datetime = NaiveDateTime::parse_from_str(&*self.time.lock().unwrap(), "%Y-%m-%d %H:%M").expect("Unable to parse datetime");
        datetime.and_utc().timestamp()
    }
}