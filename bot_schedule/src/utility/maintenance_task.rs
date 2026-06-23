use serenity::builder::*;
use serenity::model::prelude::*;
use serenity::prelude::*;
use serenity::Error;
use serenity::http::{CacheHttp, HttpError, StatusCode};
use serenity::prelude::TypeMap;
use tokio::sync::RwLock;
use chrono::Utc;
use tracing::info;

use crate::RunData;
use crate::database::remove_run;
use crate::runs::OpenOrUser;

pub async fn handle_scheduling_maintenance(cache_http: &impl CacheHttp, data: &RwLock<TypeMap>, notify_thresh: i64, delete_thresh: i64) {
    info!("Running scheduled maintenance for scheduled runs...");
    let runs = {
        let data = data.read().await;
        let runs = match data.get::<RunData>() {
            Some(a) => a,
            None => return
        };
        runs.clone()
    };

    let ts_now = Utc::now().timestamp();

    for (umsg_id, run_info) in runs.iter() {
        let msg_id = MessageId::new(*umsg_id);
        // 1: check if the run message still exists. If it doesnt, delete the associated database entry
        match &run_info.channel_id.message(cache_http, msg_id).await {
            Err(Error::Http(HttpError::UnsuccessfulRequest(req))) => {
                match req.status_code {
                    StatusCode::NOT_FOUND => {
                        info!("Message {} no longer found, removing...", umsg_id);
                        remove_run(data, umsg_id.to_string().as_str()).await;
                        {
                            let mut data = data.write().await;
                            let runs = data.get_mut::<RunData>().unwrap();
                            runs.swap_remove(umsg_id);
                        }
                        continue;
                    },
                    _ => {},
                }
            },
            _ => {},
        }

        // 2: Check if any runs are within the next X-minutes, and if so notify the assigned within the channel
        if ((run_info.time - ts_now) < notify_thresh) & ((run_info.time - ts_now) > 0) {
            // get all the assigned users
            let mut mentions = String::from("");
            for (_, v) in run_info.line_up.iter() {
                match &v.user {
                    OpenOrUser::User(a) => mentions.push_str(format!("{}", a).as_str()),
                    _ => {}
                }
            }

            // get an actual message object so that we can reply to it. Should never fail after previous if statement to unwrapping
            let msg = &run_info.channel_id.message(cache_http, msg_id).await.unwrap();
            let _ = msg.reply(
                cache_http,
                format!("{} we are forming for the run: **{}**", mentions, &run_info.name)
            ).await.unwrap();
        // 3: Check if any runs are in the past (by Y-hours) and if so, delete the post
        } else if (ts_now - run_info.time) > delete_thresh {
            info!("Message {} run time ({}) is over an hour ago from now ({}), deleting...", umsg_id, &run_info.time, &ts_now);
            let _ = &run_info.channel_id.delete_message(
                cache_http.http(),
                msg_id
            ).await.unwrap();
            remove_run(data, umsg_id.to_string().as_str()).await;
            {
                let mut data = data.write().await;
                let runs = data.get_mut::<RunData>().unwrap();
                runs.swap_remove(umsg_id);
            }
        }
    }
}