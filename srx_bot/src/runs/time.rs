use chrono::prelude::*;
use chrono_tz::Tz;


pub fn get_timestamp(time: String, timezone: Tz) -> Option<i64> {
    if let Ok(ts) = NaiveDateTime::parse_from_str(&time, "%Y-%m-%d %H:%M") {
        let t = timezone.from_local_datetime(&ts).unwrap();
        Some(t.timestamp())
    } else if let Ok(t) = NaiveTime::parse_from_str(&time, "%H:%M") {
        let ts = Local::now().date_naive().and_time(t);
        Some(ts.and_local_timezone(timezone).unwrap().timestamp())
    } else {
        None
    }
}


pub fn get_datetime(timestamp: i64, timezone: Tz) -> String {
    let t = timezone.timestamp_opt(timestamp, 0).unwrap();
    format!("{}", t.format("%Y-%m-%d %H:%M"))
}