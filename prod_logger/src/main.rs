use std::sync::Arc;
use std::thread;
use std::time::Duration;

use tracing::{info};
use tracing_subscriber::{EnvFilter, filter::LevelFilter};

use config::Config;

use tokio::signal;
use tokio::time::timeout;
use tokio::sync::{mpsc, mpsc::Receiver};

use serenity::async_trait;
use serenity::model::{gateway::Ready, channel::Message, id::ChannelId, Timestamp};
use serenity::prelude::*;
use serenity::builder::{CreateEmbed, CreateEmbedFooter, CreateMessage};
use serenity::utils::MessageBuilder;

use prod_logger::station_interaction::listen_for_prod;
use prod_logger::config::AppConfig;


struct Handler;

async fn send_prod_logs_to_discord(mut rx: Receiver<String>, ctx: Context, channel_id: ChannelId) {
    let mut mb = MessageBuilder::new();
    let mut i = 0;
    loop {
        let _ = timeout(Duration::from_millis(10000), (async || {
            while i < 10 {
                i = i + 1;
                if let Some(line) = rx.recv().await {
                    mb.push_codeblock_safe(line, Some("ansi"));
                }
            }
        })()).await;
        let resp = mb.build();
        if !resp.is_empty() {
            if let Err(why) = channel_id.say(ctx.http.clone(), &resp).await {
                println!("Error sending messsage: {why:?}");
            }
        }
        mb.0.clear();
        i = 0;
    }
}

 

#[async_trait]
impl EventHandler for Handler {
    // Set a handler for the `message` event. This is called whenever a new message is received.
    //
    // Event handlers are dispatched through a threadpool, and so multiple events can be
    // dispatched simultaneously.
    async fn message(&self, ctx: Context, msg: Message) {
        if msg.content == "!ping" {
            // Sending a message can fail, due to a network error, an authentication error, or lack
            // of permissions to post in the channel, so log to stdout when some error happens,
            // with a description of it.
            if let Err(why) = msg.channel_id.say(&ctx.http, "!Ping").await {
                println!("Error sending message: {why:?}");
            }
        } else if msg.content == "!test" {
            let footer = CreateEmbedFooter::new("Footer");
            let embed = CreateEmbed::new()
                .title("Test embed")
                .description("embed description")
                .footer(footer)
                .field("field 1 name", "field 1 value", true)
                .timestamp(Timestamp::now());
            
            let builder = CreateMessage::new()
                .content("msg content")
                .embed(embed);
            
            if let Err(why) = msg.channel_id.send_message(&ctx.http, builder).await {
                println!("Error sending message: {why:?}");
            }

        }
    }

    // Set a handler to be called on the `ready` event. This is called when a shard is booted, and
    // a READY payload is sent by Discord. This payload contains data like the current user's guild
    // Ids, current user data, private channels, and more.
    //
    // In this case, just print what the current user's username is.
    async fn ready(&self, ctx: Context, ready: Ready) {
        println!("{} is connected!", ready.user.name);

        // Moose's server :: prod_log
        let channel_id = ChannelId::new(1476998955337519297);

        let (tx, rx) = mpsc::channel(128);

        let prod_watch_handle = thread::spawn(|| {
            listen_for_prod(tx);
        });
        tokio::spawn(async move {
            send_prod_logs_to_discord(rx, ctx, channel_id).await;
        });
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
    let settings: Arc<AppConfig> = Arc::new(settings.try_deserialize().unwrap());

    // get the discord bot token
    let token = std::fs::read_to_string(".token").expect("Unable to read token file");

    // Set gateway intents, which decides what events the bot will be notified about
    let intents = GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::MESSAGE_CONTENT;

    // Create a new instance of the Client, logging in as a bot. This will automatically prepend
    // your bot token with "Bot ", which is a requirement by Discord for bot users.
    let mut client =
        Client::builder(&token, intents).event_handler(Handler).await.expect("Err creating client");
    
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