use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use tracing::{info, warn, instrument};
use tracing_subscriber::{EnvFilter, filter::LevelFilter};

use config::Config;

use tokio::signal;
use tokio::time::timeout;
use tokio::sync::{mpsc, mpsc::{Sender, Receiver}};
use tokio_util::sync::CancellationToken;

use poise::serenity_prelude as serenity; 

use prod_logger::station_interaction::listen_for_prod;
use prod_logger::ss_client_interaction::{starsonata_start, starsonata_login, get_sleep_time};
use prod_logger::config::AppConfig;


// User data, which is stored and accessible in all command invocations
struct Data {
    settings: Arc<AppConfig>,
    ss_handle: Mutex<Option<std::process::Child>>,
    ss_window_id: Mutex<Option<String>>,
    shutdown_token: CancellationToken,
}

type Error = Box<dyn std::error::Error + Send + Sync>;
type Context<'a> = poise::Context<'a, Data, Error>;


// let mut mb = serenity::MessageBuilder::new();
// mb.push_codeblock_safe(log_line, Some("ansi"));

// if let Err(why) = ctx.data().log_channel_id.unwrap().say(ctx.serenity_context().http.clone(), &mb.build()).await {
//     warn!("Error sending message: {why:?}");
// }
// let channel_id = ChannelId::new(1476998955337519297);

// let (tx, rx) = mpsc::channel(128);

// let prod_watch_handle = thread::spawn(|| {
//     listen_for_prod(tx);
// });
// tokio::spawn(async move {
//     send_prod_logs_to_discord(rx, ctx, channel_id).await;
// });


async fn send_prod_logs_to_discord(mut rx: Receiver<String>, channel_id: serenity::ChannelId, http: Arc<serenity::Http>, shutdown_token: CancellationToken) {
    // initialize a new message builder
    let mut mb = serenity::MessageBuilder::new();
    let mut i = 0; // count of messages

    // loop while we aren't cancelled
    while !shutdown_token.is_cancelled() {
        let _ = timeout(Duration::from_millis(10000), (async || {
            // want to only wait for 10 messages before sending
            while i < 10 {
                i = i + 1;
                if let Some(line) = rx.recv().await {
                    mb.push_codeblock_safe(line, Some("ansi"));
                }
            }
        })()).await;

        // build the message and send it
        let resp = mb.build();
        if !resp.is_empty() {
            if let Err(why) = channel_id.say(http.clone(), &resp).await {
                println!("Error sending messsage: {why:?}");
            }
        }
        mb.0.clear();
        i = 0;
    }
}


#[instrument(skip(ctx))]
#[poise::command(slash_command)]
async fn shutdown_all(ctx: Context<'_>) -> Result<(), Error> {
    // let the user know we are shutting down bot
    ctx.send(poise::CreateReply::default()
        .content("Recieved shutdown command, shutting bot down.")
        .ephemeral(true)
    ).await.unwrap();

    let mut handle = ctx.data().ss_handle.lock().unwrap();
    let output = match &mut *handle {
        Some(h) => {
            info!("Active Star Sonata handle, shutting down.");
            h.kill()
        },
        _ => Ok(())
    };
    drop(handle); //try to avoid poisoning if we can't kill the task
    output.expect("Unable to kill Star Sonata task");

    info!("Shutting bot down after recieving shut-down Discord command.");
    ctx.data().shutdown_token.cancel();
    Ok(())
}


#[instrument(skip(ctx))]
#[poise::command(slash_command)]
async fn start_starsonata(ctx: Context<'_>) -> Result<(), Error> {
    info!("Starting Star Sonata client");
    let utc_now = chrono::Utc::now().timestamp();
    let utc_startup = utc_now + i64::try_from(get_sleep_time(ctx.data().settings.clone())).unwrap();

    let msg_handle = ctx.send(poise::CreateReply::default()
        .content(format!("Star Sonata client should be starting <t:{}:R>", utc_startup))
        .ephemeral(true)
    ).await.unwrap();

    // split so that we dont hold the lock for the entire wait time to startup the client
    // and avoid poisoning if the startup fails
    // TODO pass errors up so that we can handle them here
    let (ss_handle, window_id) = starsonata_start(ctx.data().settings.clone());
    {
        let mut handle = ctx.data().ss_handle.lock().unwrap();
        let mut window = ctx.data().ss_window_id.lock().unwrap();
        *handle = Some(ss_handle);
        *window = window_id.clone();
    }

    // handle the login
    starsonata_login(ctx.data().settings.clone(), window_id);
    // update the message
    msg_handle.edit(ctx, poise::CreateReply::default()
        .content("Star Sonata client should have started.\nLogging in...")
        .ephemeral(true)
    ).await.expect("Unable to edit message.");

    Ok(())
}



#[poise::command(slash_command)]
async fn start_capturing_and_logging(ctx: Context<'_>) -> Result<(), Error> {
    // create the channel for prod log communication
    let (tx, rx) = mpsc::channel(128);

    // clone the data we need so it can be moved as appropriate
    let channel = ctx.data().settings.discord.prod_log_channel_id.clone();
    let http = ctx.serenity_context().http.clone();
    let c_token1 = ctx.data().shutdown_token.clone();
    let c_token2 = ctx.data().shutdown_token.clone();

    // going to use tokio spawn to handle the messages to discord, as it should be able to run on the 
    // same thread as the discord bot and not cause issues
    let send_logs_handle = tokio::spawn(async move {
        send_prod_logs_to_discord(
            rx,
            channel,
            http,
            c_token1,
        ).await
    });

    info!("Spawning thread with to capture data.");
    let prod_watch_handle = thread::spawn(|| {
        listen_for_prod(tx, c_token2);
    });
    Ok(())
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
    
    // TODO: move loading into individual functions so that they can re-load when called and 
    // update without having to re-load the full program
    // load settings
    let settings = Config::builder()
        .add_source(config::File::with_name("config/config.toml"))
        .build()
        .unwrap();
    let settings: Arc<AppConfig> = Arc::new(settings.try_deserialize().unwrap());

    // Set gateway intents, which decides what events the bot will be notified about
    let intents = serenity::GatewayIntents::non_privileged();

    // need a clone before it gets moved into the closure
    let cl_settings = settings.clone();

    // Create the shutdown notification system
    let shutdown_token = CancellationToken::new();
    let shutdown_clone = shutdown_token.clone();

    let framework = poise::Framework::builder()
        .options(
            poise::FrameworkOptions {
                commands: vec![start_capturing_and_logging(), start_starsonata(), shutdown_all()],
                ..Default::default()
        })
        .setup(move |ctx, _ready, framework| {
            Box::pin(async move {
                poise::builtins::register_in_guild(ctx, &framework.options().commands, serenity::GuildId::new(1409517559321071790)).await?;
                Ok(Data {
                    settings: cl_settings,
                    ss_handle: Mutex::new(None),
                    ss_window_id: Mutex::new(None),
                    shutdown_token: shutdown_clone,
                })
            })
        })
        .build();

    // Create a new instance of the Client, logging in as a bot. This will automatically prepend
    // your bot token with "Bot ", which is a requirement by Discord for bot users.
    let mut client = serenity::Client::builder(&settings.discord.bot_token, intents)
        .framework(framework)
        .await
        .expect("Err creating client");
    
    let shard_manager1 = client.shard_manager.clone();
    // let shard_manager2 = client.shard_manager.clone();
    // Shutdown handler for ctrl-c
    tokio::spawn(async move {
        tokio::select! {
            _ = shutdown_token.cancelled() => {shard_manager1.shutdown_all().await},
            _ = signal::ctrl_c() => {
                shutdown_token.cancel();
                shard_manager1.shutdown_all().await;
            }
        };
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