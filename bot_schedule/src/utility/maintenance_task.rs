use serenity::builder::*;
use serenity::model::prelude::*;
use serenity::prelude::*;
use chrono::Utc;
use tracing::info;

use crate::database::remove_run;

pub async fn handle_scheduling_maintenance(ctx: &Context) {
    let runs = {
        let data = ctx.data.read().await;
        let runs = data.get::<RunData>().unwrap();
        runs.clone()
    };

    let ts_now = Utc::now().timestamp();

    for (umsg_id, run_info) in runs.iter() {
        let msg_id = umsg_id.parse::<MessageId>().unwrap();
        // 1: check if the run message still exists. If it doesnt, delete the associated database entry
        match &run_info.channel_id.message(ctx, msg_id).await {
            Err(Error::Http(HttpError::UnsuccessfulRequest(req))) => {
                match req.status_code {
                    StatusCode::NOT_FOUND => {
                        info!("Message {} no longer found, removing...", umsg_id);
                        remove_run(ctx, umsg_id.to_string().as_str()).await;
                        {
                            let data = ctx.data.read().await;
                            let mut runs = data.get_mut::<RunData>().unwrap();
                            runs.swap_remove(&umsg_id);
                        }
                    },
                    _ => {},
                }
            },
            _ => {},
        }

        // 2: Check if any runs are within the next X-minutes, and if so notify the assigned within the channel
        if ((run_info.time - ts_now) < 300) & ((run_info.time - ts_now) > 0) {
            // get all the assigned users
            let mut mentions = String::from("");
            for (_, v) in &run_info.line_up.iter() {
                mentions.push_str(format!("{}", v.user).as_str())
            }

            &run_info.channel_id.send_message(
                ctx,
                CreateMessage::new()
                    .content(format!("{} we are forming for the run", mentions))
            ).await.unwrap();
        // 3: Check if any runs are in the past (by Y-hours) and if so, delete the post
        } else if (ts_now - run_info.time) > 3600 {
            info!("Message {} run time ({}) is over an hour ago from now ({}), deleting...", umsg_id, &run_info.time, &ts_now);
            &run_info.channel_id.delete_message(
                ctx,
                msg_id
            ).await.unwrap();
            remove_run(ctx, umsg_id.to_string().as_str());
            {
                let data = ctx.data.read().await;
                let mut runs = data.get_mut::<RunData>().unwrap();
                runs.swap_remove(&umsg_id);
            }
        }
    }
}