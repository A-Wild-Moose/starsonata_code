use std::sync::{LazyLock, Arc, Mutex, atomic::{AtomicBool, Ordering}};
use std::thread;
use std::time::{Duration, SystemTime};
use std::io::{Write, BufWriter, BufReader, Read};
use std::fs::File;
use regex::{Regex, RegexSet};

use tokio::signal;
use tokio::time::{sleep, timeout};
use tokio::sync::{mpsc, mpsc::{Sender, Receiver}};

use serenity::async_trait;
use serenity::model::{gateway::Ready, channel::Message, id::{GuildId, ChannelId}, Timestamp};
use serenity::prelude::*;
use serenity::builder::{CreateEmbed, CreateEmbedAuthor, CreateEmbedFooter, CreateButton, CreateMessage};
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
    transfer_credits: Regex,
    equip: Regex
}

impl Sire {
    fn new() -> Self {
        // +Shadow Wolf transferred 1 Fallen Secondary Desolation Beam* out of base
        // +Shadow Wolf transferred 1 Empyreal Incinerator* out of base
        // +Shadow Wolf transferred 1 Incineration Rocket* out of base

        // +Shadow Wolf transferred 50,000,000 credits to base
        // +Shadow Wolf took 50,000,000 credits from base
        let pats = vec![
            r"\+(?<player>[[:word:] '\-_]*) transferred (?<quant>[0-9]+) (?<item>[[:word:] '\.\-\*]*) (?<dir>(to|out of)) base",
            r"\+(?<player>[[:word:] '\-_]*) using (?<item>[[:word:] '\.\-]*) Blueprint",
            r"\+(?<player>[[:word:] '\-_]*) constructing (?<item>[[:word:] '\.\-]*)",
            r"Construction finished on (?<quant>[0-9]*) (?<item>[[:word:] '\.\-]*)",
            r"\+(?<player>[[:word:] '\-_]*) (transferred|took) (?<quant>[[0-9],]+) credits (?<dir>(to|from)) base",
            r"(?<player>[[:word:] '\-_]*) (?<dir>(un)?equipped) (?<item>[[:word:] '\-\*]*)x(?<quant>[0-9]+)."
        ];
        Self {
            set: RegexSet::new(pats.clone()).unwrap(),
            transfer: Regex::new(pats[0]).unwrap(),
            use_bp: Regex::new(pats[1]).unwrap(),
            construct: Regex::new(pats[2]).unwrap(),
            construct_done: Regex::new(pats[3]).unwrap(),
            transfer_credits: Regex::new(pats[4]).unwrap(),
            equip: Regex::new(pats[5]).unwrap()
        }
    }

    fn get_match_capture(&self, a: &str, tx: Sender<String>) {
        for cap in self.transfer.captures_iter(a) {
            let line = format!(
                "\u{001b}[0;34m{}\u{001b}[0;0m transferred \u{001b}[0;34m{}\u{001b}[0;0m \u{001b}[0;33m{}\u{001b}[0;0m {} base",
                cap.name("player").unwrap().as_str(),
                cap.name("quant").unwrap().as_str(),
                cap.name("item").unwrap().as_str(),
                cap.name("dir").unwrap().as_str()
            );
            if let Err(why) = tx.blocking_send(line) {
                println!("Unable to transmit captured line: {:?}", why);
            }
        }
        for cap in self.use_bp.captures_iter(a) {
            let line = format!(
                "\u{001b}[0;34m{}\u{001b}[0;0m using \u{001b}[0;33m{}\u{001b}[0;0m Blueprint",
                cap.name("player").unwrap().as_str(),
                cap.name("item").unwrap().as_str(),
            );
            if let Err(why) = tx.blocking_send(line) {
                println!("Unable to transmit captured line: {:?}", why);
            }
        }
        for cap in self.construct.captures_iter(a) {
            let line = format!(
                "\u{001b}[0;34m{}\u{001b}[0;0m constructing \u{001b}[0;33m{}\u{001b}[0;0m",
                cap.name("player").unwrap().as_str(),
                cap.name("item").unwrap().as_str()
            );
            if let Err(why) = tx.blocking_send(line) {
                println!("Unable to transmit captured line: {:?}", why);
            }
        }
        for cap in self.construct_done.captures_iter(a) {
            let line = format!(
                "Construction finished on \u{001b}[0;34m{}\u{001b}[0;0m \u{001b}[0;33m{}\u{001b}[0;0m",
                cap.name("quant").unwrap().as_str(),
                cap.name("item").unwrap().as_str(),
            );
            if let Err(why) = tx.blocking_send(line) {
                println!("Unable to transmit captured line: {:?}", why);
            }
        }
        for cap in self.transfer_credits.captures_iter(a) {
            let line = format!(
                "\u{001b}[0;34m{}\u{001b}[0;0m transferred \u{001b}[0;34m{}\u{001b}[0;0m credits {} base",
                cap.name("player").unwrap().as_str(),
                cap.name("quant").unwrap().as_str(),
                cap.name("dir").unwrap().as_str()
            );
            if let Err(why) = tx.blocking_send(line) {
                println!("Unable to transmit captured line: {:?}", why);
            }
        }
        for cap in self.equip.captures_iter(a) {
            let line = format!(
                "\u{001b}[0;34m{}\u{001b}[0;0m \u{001b}[0;35m{}\u{001b}[0;0m \u{001b}[0;34m{}\u{001b}[0;0m \u{001b}[0;33m{}\u{001b}[0;0m",
                cap.name("player").unwrap().as_str(),
                cap.name("dir").unwrap().as_str(),
                cap.name("quant").unwrap().as_str(),
                cap.name("item").unwrap().as_str()
            );
            if let Err(why) = tx.blocking_send(line) {
                println!("Unable to transmit captured line: {:?}", why);
            }
        }
    }
}


fn listen_for_prod(tx: Sender<String>) {
    let mut cap = get_pcap_capture();

    static SIRE: LazyLock<Sire> = LazyLock::new(|| Sire::new());

    let file = File::create("raw/raw.txt").unwrap();
    let mut wrt = BufWriter::new(&file);
    let mut i = 0;

    let now = SystemTime::now();

    loop {
        match cap.next_packet() {
            Ok(packet) => {
                let data = String::from_utf8_lossy(packet.data);
                wrt.write(format!("{} - {}\n", i, now.elapsed().unwrap().as_secs_f64()).as_bytes());
                wrt.write(&packet.data);
                wrt.write(b"\n");

                if SIRE.set.is_match(&data) {
                    SIRE.get_match_capture(&data, tx.clone());
                }
            },
            Err(_) => continue,
        };
        i += 1;
    }
}

async fn send_prod_logs(mut rx: Receiver<String>, ctx: Context, channel_id: ChannelId) {
    let mut mb = MessageBuilder::new();
    let mut i = 0;
    loop {
        // while i < 10 {
        //     i = i + 1;
        //     if let Some(line) = rx.recv().await {
        //         mb.push_mono_line_safe(line);
        //     }
        // }
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
            send_prod_logs(rx, ctx, channel_id).await;
        });
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