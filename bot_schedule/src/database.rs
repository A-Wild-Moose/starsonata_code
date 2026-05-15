use std::sync::Arc;
use std::path::Path;
use std::fs::{metadata, create_dir_all};

use rusqlite::params;
use r2d2_sqlite::SqliteConnectionManager;
use serenity::prelude::TypeMap;
use serenity::model::user::User;
use tokio::sync::RwLock;
use chrono_tz::Tz;

use crate::DbConnection;
use crate::runs::RunInfo;
use crate::runs::run_info::JoinMapExt;


pub fn get_database(path: &str) -> r2d2::Pool<SqliteConnectionManager> {
    if !metadata(path).is_ok() {
        let path = Path::new(path);
        create_dir_all(path.parent().unwrap()).unwrap();
    }

    let manager = SqliteConnectionManager::file(path);
    let pool = r2d2::Pool::new(manager).unwrap();

    // make sure the tables exist
    let conn = pool.get().unwrap();
    conn
        .execute_batch(
            "BEGIN;
            CREATE TABLE IF NOT EXISTS timezone (user INT PRIMARY KEY, timezone VARCHAR(50));
            CREATE TABLE IF NOT EXISTS runs (
                msg TEXT PRIMARY KEY,
                channel_id TEXT,
                guild_id TEXT,
                organizer TEXT,
                name TEXT,
                time INTEGER,
                size INTEGER,
                line_up TEXT,
                available TEXT
            );
            COMMIT;
            ",
        )
        .unwrap();
    pool
}


pub fn add_update_timezone(pool: &r2d2::Pool<SqliteConnectionManager>, user: &User, timezone: &Tz) {
    let conn = pool.get().unwrap();
    let _ = conn.execute(
        "INSERT OR REPLACE INTO timezone (user, timezone)
        SELECT ?1, ?2",
        params![user.id.get() as i64, timezone.name()]
    ).unwrap();
}


pub fn get_timezone(pool: &r2d2::Pool<SqliteConnectionManager>, user: &User) -> Tz {
    let conn = pool.get().unwrap();
    let res: Result<String, _> = conn.query_row(
        "SELECT timezone FROM timezone WHERE user = ?1",
        params![user.id.get() as i64],
        |row| row.get(0)
    );

    match res {
        Ok(a) => a.parse::<Tz>().unwrap(),
        Err(_) => Tz::UTC
    }
}


pub fn insert_update_runinfo(pool: &r2d2::Pool<SqliteConnectionManager>, runinfo: &RunInfo) {
    let conn = pool.get().unwrap();
    // get the json strings for the line-up/available status
    let lineup = runinfo.line_up.join();
    let available = runinfo.available.join();

    let _ = conn.execute(
        "INSERT OR REPLACE INTO runs (
            msg, channel_id, guild_id, organizer, name, time, size, line_up, available
        ) SELECT ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9",
        params![
            runinfo.msg_id.unwrap().get().to_string(),
            runinfo.channel_id.get().to_string(),
            runinfo.guild_id.unwrap().get().to_string(),
            runinfo.organizer.id.get().to_string(),
            runinfo.name,
            runinfo.time,
            runinfo.size as i64,
            lineup,
            available,
        ]
    ).unwrap();
}


// pub async fn load_runinfo(pool: &r2d2::Pool<SqliteConnectionManager>) -> IndexMap<u64, RunInfo> {

// }