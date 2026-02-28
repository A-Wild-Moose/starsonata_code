use std::sync::{LazyLock, Arc, atomic::{AtomicBool, Ordering}};
use std::io::{Write, BufWriter, BufReader, Read};
use std::fs::File;
use regex::Regex;
// use once_cell::sync::Lazy;

use tokio::signal;

use serenity::async_trait;
use serenity::model::{channel::Message, id::{GuildId, ChannelId}};
use serenity::model::gateway::Ready;
use serenity::prelude::*;
use serenity::utils::MessageBuilder;

struct Handler;

use prod_logger::device::get_pcap_capture;


fn main1() {

    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();

    ctrlc::set_handler(move || {
        r.store(false, Ordering::SeqCst);
    }).expect("Error setting Ctrl-C handler");
    
    let mut cap = get_pcap_capture();

    let file = File::create("raw/raw.txt").unwrap();
    let mut wrt = BufWriter::new(&file);

    while running.load(Ordering::SeqCst) {
        let packet = cap.next_packet().unwrap();
        let data = String::from_utf8_lossy(packet.data);
        
        wrt.write(&packet.data).unwrap();
    }
}

async fn listen_for_prod(ctx: Context, channel_id: ChannelId) {
    let file = File::open("raw/raw_transfer_equip.txt").unwrap();
    let mut rdr = BufReader::new(file);

    let mut buf: Vec<u8> = vec![];

    let _ = rdr.read_to_end(&mut buf).unwrap();
    let data = String::from_utf8_lossy(&buf);

    static RE_TRANSFER_TO: LazyLock<Regex> = LazyLock::new(|| Regex::new(
        r"\+(?<player>[[:word:] '-_]*) transferred (?<quant>[0-9]?) (?<item>[[:word:] '-]*) to base"
    ).unwrap());
    static RE_TRANSFER_OUT: LazyLock<Regex> = LazyLock::new(|| Regex::new(
        r"\+(?<player>[[:word:] '-_]*) transferred (?<quant>[0-9]?) (?<item>[[:word:] '-]*) out of base"
    ).unwrap());
    static RE_USING_BP: LazyLock<Regex> = LazyLock::new(|| Regex::new(
        r"\+(?<player>[[:word:] '-_]*) using (?<item>[[:word:] '-]*) Blueprint"
    ).unwrap());
    static RE_CONSTRUCTING: LazyLock<Regex> = LazyLock::new(|| Regex::new(
        r"\+(?<player>[[:word:] '-_]*) constructing (?<item>[[:word:] '-]*)\x00"
    ).unwrap());
    static RE_CONSTRUCTION_DONE: LazyLock<Regex> = LazyLock::new(|| Regex::new(
        r"\+Construction finished on (?<quant>[0-9]?) (?<item>[[:word:] '-]*)\x00"
    ).unwrap());
    static RE_CREDITS_ADD: LazyLock<Regex> = LazyLock::new(|| Regex::new(
        r"\+(?<player>[[:word:] '-_]*) transferred (?<quant>[[0-9],]?) credits to base"
    ).unwrap());
    static RE_CREDITS_TAKE: LazyLock<Regex> = LazyLock::new(|| Regex::new(
        r"\+(?<player>[[:word:] '-_]*) took (?<quant>[[0-9],]?) credits from base"
    ).unwrap());

    for cap in RE_TRANSFER_TO.captures_iter(&data){
        // let tt = cap.get_match().as_str();
        let player = cap.name("player").unwrap().as_str();
        let quant = cap.name("quant").unwrap().as_str();
        let item = cap.name("item").unwrap().as_str();

        let resp = MessageBuilder::new()
            .push_bold_safe(player)
            .push(" transferred ")
            .push_italic_safe(quant)
            .push(" ")
            .push_italic_safe(item)
            .push(" to base")
            .build();
        
        if let Err(why) = channel_id.say(&ctx.http, &resp).await {
            println!("Error sending messsage: {why:?}");
        }
        
    }
    for cap in RE_TRANSFER_OUT.captures_iter(&data){
        // let to = cap.get_match().as_str();
        let player = cap.name("player").unwrap().as_str();
        let quant = cap.name("quant").unwrap().as_str();
        let item = cap.name("item").unwrap().as_str();

        let resp = MessageBuilder::new()
            .push_bold_safe(player)
            .push(" transferred ")
            .push_italic_safe(quant)
            .push(" ")
            .push_italic_safe(item)
            .push(" out of base")
            .build();
        
        if let Err(why) = channel_id.say(&ctx.http, &resp).await {
            println!("Error sending messsage: {why:?}");
        }
        
    }
    for cap in RE_USING_BP.captures_iter(&data){
        // let ubp = cap.get_match().as_str();
        let player = cap.name("player").unwrap().as_str();
        let item = cap.name("item").unwrap().as_str();

        let resp = MessageBuilder::new()
            .push_bold_safe(player)
            .push(" using ")
            .push_italic_safe(item)
            .push(" Blueprint")
            .build();
        
        if let Err(why) = channel_id.say(&ctx.http, &resp).await {
            println!("Error sending messsage: {why:?}");
        }
        
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
            if let Err(why) = msg.channel_id.say(&ctx.http, "Pong!").await {
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

        let mut guild_id_: Option<GuildId> = None;
        for gid in ready.guilds {
            let guild_name = gid.id.get_preview(&ctx.http).await.unwrap().name;
            println!("{:?}", guild_name);
            if guild_name == "Moose's server"{
                guild_id_ = Some(gid.id);
            }
        }
        let guild_id = guild_id_.expect("Specified guild not found.");

        let channels_ = guild_id.channels(&ctx.http).await.unwrap();
        let (channel_id, channel) = channels_.iter().find(|&(_, x)| x.name == "prod_log").expect("Specified channel not found");

        listen_for_prod(ctx, *channel_id).await;
    }
}

#[tokio::main]
async fn main() {
    let file = File::open(".token").unwrap();
    let mut rdr = BufReader::new(file);
    let mut buf: Vec<u8> = vec![];

    let _ = rdr.read_to_end(&mut buf).unwrap();
    let token = String::from_utf8_lossy(&buf);

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
    if let Err(why) = client.start().await {
        println!("Client error: {why:?}");
    }
}