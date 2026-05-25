use std::sync::{Arc};
// use std::thread;

use tokio::signal;
use tracing_subscriber::{EnvFilter, filter::LevelFilter};

use serenity::async_trait;
use serenity::builder::{CreateInteractionResponse, CreateInteractionResponseMessage};
use serenity::model::application::Interaction;
use serenity::model::gateway::Ready;
use serenity::model::id::GuildId;
use serenity::prelude::*;

use indexmap::IndexMap;

use r2d2_sqlite::SqliteConnectionManager;

use config::Config;

mod commands;
mod runs;
mod database;
mod interactions;
mod error;

#[derive(serde::Deserialize, Debug)]
struct ConfigDiscord {
    bot_token: String
}

#[derive(serde::Deserialize, Debug)]
struct ConfigApp {
    discord: ConfigDiscord
}

// Handle user/custom data
struct DbConnection;
impl TypeMapKey for DbConnection {
    type Value = r2d2::Pool<SqliteConnectionManager>;
}

struct RunData;
impl TypeMapKey for RunData {
    type Value = IndexMap<u64, runs::RunInfo>;
}

struct ClassData;
impl TypeMapKey for ClassData {
    type Value = IndexMap<String, runs::EmojiData>;
}

struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        if let Interaction::Command(command) = interaction {
            // println!("Received command interaction: {command:#?}");

            let content = match command.data.name.as_str() {
                "custom" => {
                    commands::schedule_custom::run(&ctx, &command).await.unwrap();
                    None
                },
                "set_timezone" => {
                    commands::set_timezone::run(&ctx, &command).await.unwrap();
                    None
                }
                _ => Some("not implemented :(".to_string()),
            };

            if let Some(content) = content {
                let data = CreateInteractionResponseMessage::new().content(content);
                let builder = CreateInteractionResponse::Message(data);
                if let Err(why) = command.create_response(&ctx.http, builder).await {
                    println!("Cannot respond to slash command: {why}");
                }
            }
        } else if let Interaction::Component(component) = interaction {
            let class_data = {
                let data = ctx.data.read().await;
                data.get::<ClassData>().unwrap().clone()
            };
            let custom_id = component.data.custom_id.clone();
            if &custom_id == "edit_button" {
                interactions::edit::handle_edit(&ctx, &component).await;
            } else if class_data.contains_key(&custom_id) {
                interactions::update_available::handle_update_available(&ctx, &component).await;
            }
        }
    }

    async fn ready(&self, ctx: Context, ready: Ready) {
        let guild_id = GuildId::new(1409517559321071790);

        let _ = guild_id
            .set_commands(&ctx.http, vec![
                commands::schedule_custom::register(),
                commands::set_timezone::register(),
            ])
            .await;
        
        // add the data on runs
        let runs_info = database::load_runinfo(&ctx, &ctx.data).await;
        {
            let mut data = ctx.data.write().await;
            data.insert::<RunData>(runs_info);
        }
    }
}

#[tokio::main]
async fn main() {
    // logging setup
    let filter = EnvFilter::builder()
        .with_default_directive(LevelFilter::INFO.into())
        .from_env()
        .unwrap();
    
    let _subscriber = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .init();
    
    // load settings
    let settings = Config::builder()
        .add_source(config::File::with_name("config/config.toml"))
        .build()
        .unwrap();
    let settings: Arc<ConfigApp> = Arc::new(settings.try_deserialize().unwrap());

    // static stuff
    let mut ss_classes: IndexMap<String, runs::EmojiData> = IndexMap::with_capacity(8);
    ss_classes.insert("Speed Demon".to_string(), runs::EmojiData{name: "sd".to_string(), id: 1488700320484823201});
    ss_classes.insert("Seer".to_string(), runs::EmojiData{name: "seer".to_string(), id: 1488700355888939069});
    ss_classes.insert("Berserker".to_string(), runs::EmojiData{name: "zerk".to_string(), id: 1488700391548784650});
    ss_classes.insert("Ranger".to_string(), runs::EmojiData{name: "ranger".to_string(), id: 1488700421676470282});
    ss_classes.insert("Shield Monkey".to_string(), runs::EmojiData{name: "shm".to_string(), id: 1488700454811734177});
    ss_classes.insert("Engineer".to_string(), runs::EmojiData{name: "engineer".to_string(), id: 1488700487451803750});
    ss_classes.insert("Gunner".to_string(), runs::EmojiData{name: "gunner".to_string(), id: 1488700519055626330});
    ss_classes.insert("Fleet Commander".to_string(), runs::EmojiData{name: "fc".to_string(), id: 1490107303448416370});


    // Build our client.
    let mut client = Client::builder(&settings.discord.bot_token, GatewayIntents::empty())
        .event_handler(Handler)
        .await
        .expect("Error creating client");
    // set custom data
    {
        let mut data = client.data.write().await;
        data.insert::<DbConnection>(database::get_database("raw/runs.db"));
        data.insert::<ClassData>(ss_classes);
    }
    
    // Handle shutdowns gracefully
    let shard_manager = client.shard_manager.clone();
    tokio::spawn(async move {
        signal::ctrl_c().await.unwrap();
        shard_manager.shutdown_all().await;
    });

    // Finally, start a single shard, and start listening to events.
    //
    // Shards will automatically attempt to reconnect, and will perform exponential backoff until
    // it reconnects.
    if let Err(why) = client.start().await {
        println!("Client error: {why:?}");
    }
}