use std::sync::{LazyLock, Arc, Mutex, atomic::{AtomicBool, Ordering}};
use std::time::Duration;
use std::io::{Write, BufWriter, BufReader, Read};
use std::fs::File;
use regex::{Regex, RegexSet};

use tokio::signal;
use tokio::time::sleep;

use serenity::async_trait;
use serenity::model::{channel::Message, id::{GuildId, ChannelId}};
use serenity::model::gateway::Ready;
use serenity::http::Http;
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

struct Sire { // Station Interaction REgex
    set: RegexSet,
    transfer: Regex,
    use_bp: Regex,
    construct: Regex,
    construct_done: Regex,
    transfer_credits: Regex
}

impl Sire {
    fn new() -> Self {
        let pats = vec![
            r"\+(?<player>[[:word:] '\-_]*) transferred (?<quant>[0-9]+) (?<item>[[:word:] '\-]*) (?<dir>(to|out of)) base",
            r"\+(?<player>[[:word:] '\-_]*) using (?<item>[[:word:] '\-]*) Blueprint",
            r"\+(?<player>[[:word:] '\-_]*) constructing (?<item>[[:word:] '\-]*)\x00",
            r"\+Construction finished on (?<quant>[0-9]?) (?<item>[[:word:] '\-]*)\x00",
            r"\+(?<player>[[:word:] '\-_]*) transferred (?<quant>[[0-9],]?) credits (?<dir>(to|from)) base"
        ];
        Self {
            set: RegexSet::new(pats.clone()).unwrap(),
            transfer: Regex::new(pats[0]).unwrap(),
            use_bp: Regex::new(pats[1]).unwrap(),
            construct: Regex::new(pats[2]).unwrap(),
            construct_done: Regex::new(pats[3]).unwrap(),
            transfer_credits: Regex::new(pats[4]).unwrap()
        }
    }

    async fn get_match_capture(&self, a: &str, mb: Arc<Mutex<MessageBuilder>>) {
        for cap in self.transfer.captures_iter(a) {
            let mut mb = mb.lock().unwrap();
            mb.push_mono_line_safe(format!(
                "{} transferred {} {} {} base",
                cap.name("player").unwrap().as_str(),
                cap.name("quant").unwrap().as_str(),
                cap.name("item").unwrap().as_str(),
                cap.name("dir").unwrap().as_str()
            ));
        }
        for cap in self.use_bp.captures_iter(a) {
            let mut mb = mb.lock().unwrap();
            mb.push_mono_line_safe(format!(
                "{} using {} Blueprint",
                cap.name("player").unwrap().as_str(),
                cap.name("item").unwrap().as_str(),
            ));
        }
        for cap in self.construct.captures_iter(a) {
            let mut mb = mb.lock().unwrap();
            mb.push_mono_line_safe(format!(
                "{} constructing {}",
                cap.name("player").unwrap().as_str(),
                cap.name("item").unwrap().as_str()
            ));
        }
        for cap in self.construct_done.captures_iter(a) {
            let mut mb = mb.lock().unwrap();
            mb.push_mono_line_safe(format!(
                "Construction finished on {} {}",
                cap.name("quant").unwrap().as_str(),
                cap.name("item").unwrap().as_str(),
            ));
        }
        for cap in self.transfer_credits.captures_iter(a) {
            let mut mb = mb.lock().unwrap();
            mb.push_mono_line_safe(format!(
                "{} transferred {} credits {} base",
                cap.name("player").unwrap().as_str(),
                cap.name("quant").unwrap().as_str(),
                cap.name("dir").unwrap().as_str()
            ));
        }
    }
}


async fn listen_for_prod(mb: Arc<Mutex<MessageBuilder>>) {
    let file = File::open("raw/raw_transfer_equip.txt").unwrap();
    let mut rdr = BufReader::new(file);

    let mut buf: Vec<u8> = vec![];

    let _ = rdr.read_to_end(&mut buf).unwrap();
    let data = String::from_utf8_lossy(&buf);

    static SIRE: LazyLock<Sire> = LazyLock::new(|| Sire::new());

    if SIRE.set.is_match(&data) {
        SIRE.get_match_capture(&data, mb).await;
    }
}

async fn send_prod_logs(mb: Arc<Mutex<MessageBuilder>>, cache_http: Arc<Http>, channel_id: &ChannelId) {
    let resp = {
        let mut mb = mb.lock().unwrap();
        let r = mb.build();
        mb.0.clear();
        r
    };
    if let Err(why) = channel_id.say(cache_http, &resp).await {
        println!("Error sending messsage: {why:?}");
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

        let mb: Arc<Mutex<MessageBuilder>> = Arc::new(Mutex::new(MessageBuilder::new()));

        listen_for_prod(mb.clone()).await;
        loop {
            sleep(Duration::from_millis(3000)).await;
            send_prod_logs(mb.clone(), ctx.http.clone(), channel_id).await;
        }
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