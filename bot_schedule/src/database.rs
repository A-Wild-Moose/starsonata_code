use std::path::Path;
use std::fs::{metadata, create_dir_all};

use rusqlite::{params, Params, Row};
use r2d2_sqlite::SqliteConnectionManager;
use serenity::prelude::TypeMap;
use serenity::client::*;
use serenity::model::prelude::*;
use serenity::Error;
use serenity::http::{HttpError, StatusCode};
use tokio::sync::RwLock;
use chrono_tz::Tz;
use indexmap::{IndexMap, IndexSet};

// use tracing::info;

use crate::DbConnection;
use crate::runs::{RunInfo, OpenOrUser, SpotData, EmojiData};
use crate::runs::run_info::JoinMapExt;
use crate::error::BotError;


async fn parse_line_up(ctx: &Context, s: String) -> IndexMap<usize, SpotData> {
    let mut line_up = IndexMap::with_capacity(10);

    for (i, s_spot) in s.split("\n").enumerate() {
        let (emoji, user) = s_spot.split_once(": ").unwrap();
        let spot_data = if user == "<open>" {
            SpotData::default()
        } else {
            let user = user.replace(&['<', '>', '@'], "");
            let (_, s_emoji_id) = emoji.rsplit_once(":").unwrap();
            let s_emoji_id = s_emoji_id.replace(">", "");
            SpotData {
                user: OpenOrUser::User(UserId::new(user.parse::<u64>().unwrap()).to_user(ctx).await.unwrap()),
                emoji: ReactionType::from(EmojiId::new(s_emoji_id.parse::<u64>().unwrap())),
            }
        };
        line_up.insert(i as usize, spot_data);
    }
    line_up
}


async fn parse_available(ctx: &Context, s: String) -> IndexMap<User, IndexSet<EmojiData>> {
    IndexMap::with_capacity(10)
}


struct DbRunInfo {
    msg_id: String,
    channel_id: String,
    guild_id: String,
    organizer: String,
    name: String,
    time: i64,
    size: i64,
    line_up: String,
    available: String,
}


impl DbRunInfo {
    fn from_runinfo(rinfo: &RunInfo) -> Result<Self, BotError> {
        let s_msg_id = match rinfo.msg_id {
            Some(mid) => mid.get().to_string(),
            None => return Err(BotError::MissingMessageError)
        };

        Ok(Self {
            msg_id: s_msg_id,
            channel_id: rinfo.channel_id.get().to_string(),
            guild_id: rinfo.guild_id.unwrap().get().to_string(),
            organizer: rinfo.organizer.id.get().to_string(),
            name: rinfo.name.clone(),
            time: rinfo.time.clone(),
            size: rinfo.size as i64,
            line_up: rinfo.line_up.join(),
            available: rinfo.available.join(),
        })
    }

    fn to_params(&self) -> (String, String, String, String, String, i64, i64, String, String) {
        (
            self.msg_id.clone(),
            self.channel_id.clone(),
            self.guild_id.clone(),
            self.organizer.clone(),
            self.name.clone(),
            self.time,
            self.size,
            self.line_up.clone(),
            self.available.clone(),
        )
    }

    fn from_row(row: &Row<'_>) -> Result<Self, rusqlite::Error> {
        Ok(Self {
            msg_id: row.get(0)?,
            channel_id: row.get(1)?,
            guild_id: row.get(2)?,
            organizer: row.get(3)?,
            name: row.get(4)?,
            time: row.get(5)?,
            size: row.get(6)?,
            line_up: row.get(7)?,
            available: row.get(8)?,
        })
    }

    async fn to_runinfo(&self, ctx: &Context) -> Result<RunInfo, BotError> {
        let msg_id = self.msg_id.parse::<MessageId>().unwrap();
        let channel_id = self.channel_id.parse::<ChannelId>().unwrap();

        // check if the message exists
        match channel_id.message(ctx, msg_id).await {
            Err(Error::Http(HttpError::UnsuccessfulRequest(req))) => {
                match req.status_code {
                    StatusCode::NOT_FOUND => return Err(BotError::MessageNotFoundError(self.msg_id)),
                    _ => {},
                }
            },
            _ => {},
        }

        Ok(RunInfo {
            msg_id: Some(msg_id),
            channel_id: channel_id,
            guild_id: Some(self.guild_id.parse::<GuildId>().unwrap()),
            organizer: self.organizer.parse::<UserId>().unwrap().to_user(ctx).await.unwrap(),
            name: self.name.clone(),
            time: self.time,
            size: self.size as usize,
            line_up: parse_line_up(ctx, self.line_up).await,
            available: parse_available(ctx, self.available).await,
        })
    }
}


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
                msg_id TEXT PRIMARY KEY,
                channel_id TEXT,
                guild_id TEXT,
                organizer_id TEXT,
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


pub async fn add_update_timezone(data: &RwLock<TypeMap>, user: &User, timezone: &Tz) {
    let data = data.read().await;
    let pool = data.get::<DbConnection>().unwrap();
    let conn = pool.get().unwrap();
    let _ = conn.execute(
        "INSERT OR REPLACE INTO timezone (user, timezone)
        SELECT ?1, ?2",
        params![user.id.get() as i64, timezone.name()]
    ).unwrap();
}


pub async fn get_timezone(data: &RwLock<TypeMap>, user: &User) -> Tz {
    let data = data.read().await;
    let pool = data.get::<DbConnection>().unwrap();
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


pub async fn insert_update_runinfo(data: &RwLock<TypeMap>, runinfo: &RunInfo) {
    let data = data.read().await;
    let pool = data.get::<DbConnection>().unwrap();
    let conn = pool.get().unwrap();

    // get the correct types to send to the database
    let db_runinfo = DbRunInfo::from_runinfo(runinfo);

    let _ = conn.execute(
        "INSERT OR REPLACE INTO runs (
            msg_id, channel_id, guild_id, organizer_id, name, time, size, line_up, available
        ) SELECT ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9",
        db_runinfo.to_params()
    ).unwrap();
}


pub async fn load_runinfo(ctx: &Context, data: &RwLock<TypeMap>) -> IndexMap<u64, RunInfo> {
    let data = data.read().await;
    let pool = data.get::<DbConnection>().unwrap();
    let conn = pool.get().unwrap();

    // setup to get the data
    let mut stmt = conn.prepare("SELECT * FROM runs").unwrap();
    let res = stmt.query_map([], DbRunInfo::from_row).unwrap();
    
    // return
    let runs: IndexMap<u64, RunInfo> = IndexMap::with_capacity(10);

    // iterate over rows
    for db_ri in res {
        match db_ri {
            Ok(db_runinfo) => {
                match db_runinfo.to_runinfo(ctx).await {
                    Ok(runinfo) => {runs.insert(runinfo.msg_id.get(), runinfo);},
                    Err(_) => {
                        // delete from the database
                    }
                }
            },
            _ => {}
        }
    }

    runs
}