use std::process::{Command, Child};
use std::{thread, time};
use std::sync::{Arc, Mutex, atomic::{AtomicBool, Ordering}};
use std::rc::Rc;
use std::cell::RefCell;

use config::Config;
use enigo::{Key, Keyboard, Enigo, Settings};
use enigo::Direction::{Click, Press, Release};
use tracing;
use tracing_subscriber;
use secrecy::{SecretBox, ExposeSecret};

#[derive(serde::Deserialize, Debug)]
struct StarSonataStartup {
    ss_path: String,
    initial_sleep: u64,
    client_load_sleep: u64,
    character_load_sleep: u64,
}

// .wine/drive_c/users/ubuntu/AppData/Roaming/Star\ Sonata\ 2/Star\ Sonata.exe

#[derive(serde::Deserialize, Debug)]
struct AppConfig {
    username: String,
    password: SecretBox<String>,
    starsonatastartup: StarSonataStartup,
}

#[tracing::instrument]
fn ss_start(enigo: Rc<RefCell<Enigo>>, settings: Rc<AppConfig>) -> Child {
    // let mut handle = Command::new(&settings.starsonatastartup.ss_path).spawn().expect("Unable to start exe");
    let handle = if cfg!(target_os="linux") {
        Command::new("wine")
            .arg(&settings.starsonatastartup.ss_path)
            .env("DISPLAY", ":0.0")
            .spawn()
            .expect("Unable to start exe")
    } else {
        let h = Command::new(&settings.starsonatastartup.ss_path)
            .spawn()
            .expect("Unable to start exe");
        
        thread::sleep(time::Duration::from_millis(settings.starsonatastartup.initial_sleep));

        tracing::info!("waited {}s starting SS client from options menu screen.", &settings.starsonatastartup.initial_sleep);
        let mut enigo = enigo.borrow_mut();
        let _ = enigo.key(Key::Return, Click);
        h
    };

    // wait for the client to load
    thread::sleep(time::Duration::from_millis(settings.starsonatastartup.client_load_sleep));
    tracing::info!("Waited {}s for the client to load, moving to login.", &settings.starsonatastartup.client_load_sleep);
    
    if cfg!(target_os="linux") {
        // let output = Command::new("xdotool")
        //     .arg("getwindowfocus")
        //     .arg("getwindowname")
        //     .env("DISPLAY", ":0.0")
        //     .output()
        //     .unwrap();
        // println!("Window focus: {:?}", output);
        let output = Command::new("xdotool")
            .args(["search", "--name", "Sonata"])
            .output()
            .unwrap();
            println!("search: {:?}, process id: {}", output, handle.id());
    }


    return handle;
}

#[tracing::instrument]
fn ss_login(enigo: Rc<RefCell<Enigo>>, settings: Rc<AppConfig>) {
    let mut enigo = enigo.borrow_mut();
    // Should be on the login screen here with cursor selecting the username field
    // First, select existing username, remove and then retype
    tracing::info!("Setting username");
    let _ = enigo.key(Key::Control, Press);
    let _ = enigo.key(Key::Unicode('a'), Click);
    let _ = enigo.key(Key::Control, Release);
    let _ = enigo.key(Key::Delete, Click);

    enigo.text(&settings.username).unwrap();

    // move onto password
    tracing::info!("Setting password");
    let _ = enigo.key(Key::Tab, Click);
    let _ = enigo.key(Key::Control, Press);
    let _ = enigo.key(Key::Unicode('a'), Click);
    let _ = enigo.key(Key::Control, Release);
    let _ = enigo.key(Key::Delete, Click);

    enigo.text(settings.password.expose_secret()).unwrap();
    enigo.key(Key::Return, Click).unwrap();

    // wait for the characters to load
    tracing::info!("Waiting for character screen to load");
    thread::sleep(time::Duration::from_millis(settings.starsonatastartup.character_load_sleep));
    enigo.text(settings.password.expose_secret()).unwrap();
    enigo.key(Key::Return, Click).unwrap();
}


fn main() {
    // load config
    let settings = Config::builder()
            .add_source(config::File::with_name("config/config.toml"))
            .build()
            .unwrap();
    let settings: Rc<AppConfig> = Rc::new(settings.try_deserialize().unwrap());

    // logging setup
    let filter = tracing_subscriber::EnvFilter::builder()
        .with_default_directive(tracing_subscriber::filter::LevelFilter::INFO.into())
        .from_env()
        .unwrap()
        .add_directive("run_ss_test=debug".parse().unwrap());

    let _subscriber = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .init();
    
    // let enigo = Arc::new(Mutex::new(Enigo::new(&Settings::default()).unwrap()));
    let mut enigo_settings = Settings::default();
    enigo_settings.x11_display = Some(":0.0".to_string());
    let enigo = Rc::new(RefCell::new(Enigo::new(&enigo_settings).unwrap()));

    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();

    ctrlc::set_handler(move || {
        r.store(false, Ordering::SeqCst);
    }).expect("Error setting Ctrl-C handler");

    let mut handle = ss_start(enigo.clone(), settings.clone());

    ss_login(enigo.clone(), settings.clone());

    while running.load(Ordering::SeqCst) {}

    handle.kill().unwrap();
    // thread::sleep(time::Duration::from_millis(15000));
    // match handle.kill() {
    //     Ok(a) => println!("{:?}", a),
    //     Err(a) => println!("{:?}", a),
    // }
}
