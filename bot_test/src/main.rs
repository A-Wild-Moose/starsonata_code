use std::sync::{Arc, Mutex};
// use std::thread;

use tokio::signal;
use tracing::{info};
use tracing_subscriber::{EnvFilter, filter::LevelFilter};
use indexmap::{IndexMap, IndexSet};
use poise::serenity_prelude as serenity;
// use chrono::prelude::*;

use config::Config;

mod runs;
mod schedule;

use crate::schedule::schedule;
use crate::runs::EmojiData;


type Error = Box<dyn std::error::Error + Send + Sync>;
type Context<'a> = poise::Context<'a, Data, Error>;
// User data, which is stored and accessible in all command invocations
pub struct Data {
    ss_classes: IndexMap<String, EmojiData>
} 


 #[derive(serde::Deserialize, Debug)]
struct ConfigDiscord {
    bot_token: String
}

#[derive(serde::Deserialize, Debug)]
struct ConfigApp {
    discord: ConfigDiscord
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
    let mut ss_classes: IndexMap<String, EmojiData> = IndexMap::with_capacity(8);
    ss_classes.insert("Speed Demon".to_string(), EmojiData{name: "sd".to_string(), id: 1488700320484823201});
    ss_classes.insert("Seer".to_string(), EmojiData{name: "seer".to_string(), id: 1488700355888939069});
    ss_classes.insert("Berserker".to_string(), EmojiData{name: "zerk".to_string(), id: 1488700391548784650});
    ss_classes.insert("Ranger".to_string(), EmojiData{name: "ranger".to_string(), id: 1488700421676470282});
    ss_classes.insert("Shield Monkey".to_string(), EmojiData{name: "shm".to_string(), id: 1488700454811734177});
    ss_classes.insert("Engineer".to_string(), EmojiData{name: "engineer".to_string(), id: 1488700487451803750});
    ss_classes.insert("Gunner".to_string(), EmojiData{name: "gunner".to_string(), id: 1488700519055626330});
    ss_classes.insert("Fleet Commander".to_string(), EmojiData{name: "fc".to_string(), id: 1490107303448416370});

    // Set gateway intents, which decides what events the bot will be notified about
    let intents = serenity::GatewayIntents::non_privileged();

    // Framework setup
    let framework = poise::Framework::builder()
        .options(
            poise::FrameworkOptions {
                commands: vec![schedule()],
                ..Default::default()
        })
        .setup(|ctx, _ready, framework| {
            Box::pin(async move {
                poise::builtins::register_in_guild(ctx, &framework.options().commands, serenity::GuildId::new(1409517559321071790)).await?;
                Ok(Data {
                    ss_classes: ss_classes
                })
            })
        })
        .build();

    // Create a new instance of the Client, logging in as a bot. This will automatically prepend
    // your bot token with "Bot ", which is a requirement by Discord for bot users.
    let mut client =
        serenity::Client::builder(&settings.discord.bot_token, intents)
            .framework(framework)
            .await
            .expect("Err creating client");
    
    let shard_manager = client.shard_manager.clone();
    
    // Shutdown handler
    tokio::spawn(async move {
        signal::ctrl_c().await.unwrap();
        shard_manager.shutdown_all().await;
    });

    // Finally, start a single shard, and start listening to events.
    //
    // Shards will automatically attempt to reconnect, and will perform exponential backoff until
    // it reconnects.
    info!("Starting discord bot client");
    if let Err(why) = client.start().await {
        println!("Client error: {why:?}");
    }
}