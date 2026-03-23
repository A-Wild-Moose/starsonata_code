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

#[cfg(not(target_os = "linux"))]
#[tracing::instrument(skip(settings))]
fn ss_start(enigo: Rc<RefCell<Enigo>>, settings: Rc<AppConfig>) -> (Child, Option<String>) {
    let handle = Command::new(&settings.starsonatastartup.ss_path)
        .spawn()
        .expect("Unable to start exe");
        
    thread::sleep(time::Duration::from_millis(settings.starsonatastartup.initial_sleep));

    tracing::info!("waited {}s starting SS client from options menu screen.", &settings.starsonatastartup.initial_sleep);
    let mut enigo = enigo.borrow_mut();
    let _ = enigo.key(Key::Return, Click);

    // wait for the client to load
    thread::sleep(time::Duration::from_millis(settings.starsonatastartup.client_load_sleep));
    tracing::info!("Waited {}s for the client to load, moving to login.", &settings.starsonatastartup.client_load_sleep);

    return (handle, None);
}

#[cfg(target_os = "linux")]
#[tracing::instrument(skip(settings))]
fn ss_start(_: Rc<RefCell<Enigo>>, settings: Rc<AppConfig>) -> (Child, Option<String>) {
    // let handle = Command::new("wine")
    //         .arg(&settings.starsonatastartup.ss_path)
    //         .env("DISPLAY", ":0.0")
    //         .spawn()
    //         .expect("Unable to start exe");
    let handle = Command::new("xvfb-run")
        .args(["-f", "~/.xauth", "-n", "99", "wine", &settings.starsonatastartup.ss_path])
        .spawn()
        .expect("Unable to start exe")

    // wait for the client to load
    thread::sleep(time::Duration::from_millis(settings.starsonatastartup.client_load_sleep));
    tracing::info!("Waited {}s for the client to load, moving to login.", &settings.starsonatastartup.client_load_sleep);
    
    // first search for the star sonata window
    let output = Command::new("xdotool")
        .args(["search", "--name", ".*Star Sonata.*"])
        .env("DISPLAY", ":99.0")
        .env("XAUTHORITY", "~/.xauth")
        .output()
        .expect("Unable to search for Star Sonata window.");
    let window = String::from_utf8_lossy(&output.stdout).trim_end().to_string();
    tracing::info!("Found window id: {:?} for Star Sonata.", window);

    return (handle, Some(window));
}

#[cfg(not(target_os = "linux"))]
#[tracing::instrument(skip(settings))]
fn ss_login(enigo: Rc<RefCell<Enigo>>, settings: Rc<AppConfig>, window: Option<String>) {
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
    
    enigo.key(Key::Return, Click).unwrap();
}

#[cfg(target_os = "linux")]
#[tracing::instrument(skip(settings))]
fn ss_login(_: Rc<RefCell<Enigo>>, settings: Rc<AppConfig>, window: Option<String>) {
    let window = window.expect("Window id was not set properly");

    // first search for the star sonata window
    let output = Command::new("xdotool")
        .args(["search", "--name", ".*Star Sonata.*"])
        // .env("DISPLAY", ":0.0")
        .env("DISPLAY", ":99.0")
        .env("XAUTHORITY", "~/.xauth")
        .output()
        .expect("Unable to search for Star Sonata window.");
    let window = String::from_utf8_lossy(&output.stdout).trim_end().to_string();
    tracing::info!("Found window id: {:?} for Star Sonata.", window);

    // try focusing on window first?
    let out = Command::new("xdotool")
        .args(["windowfocus", "--sync", &window])
        // .env("DISPLAY", ":0.0")
        .env("DISPLAY", ":99.0")
        .env("XAUTHORITY", "~/.xauth")
        .output()
        .expect("Unable to focus on window");
    tracing::debug!("Focus window output: {:?}", out);
    // Should be on the first login screen here. Cursor should be selecting the username field
    // select the username to re-type
    let out = Command::new("xdotool")
        .args(["key", "--window", &window, "ctrl+a"])
        // .env("DISPLAY", ":0.0")
        .env("DISPLAY", ":99.0")
        .env("XAUTHORITY", "~/.xauth")
        .output()
        .expect("Unable to select username");
    tracing::debug!("Select username output: {:?}", out);

    let out = Command::new("xdotool")
        .args(["key", "--window", &window, "Delete"])
        // .env("DISPLAY", ":0.0")
        .env("DISPLAY", ":99.0")
        .env("XAUTHORITY", "~/.xauth")
        .output()
        .expect("Unable to delete username");
    tracing::debug!("Delete username output: {:?}", out);
    
    tracing::info!("Typing username");
    let out = Command::new("xdotool")
        .args(["type", "--window", &window, &settings.username])
        // .env("DISPLAY", ":0.0")
        .env("DISPLAY", ":99.0")
        .env("XAUTHORITY", "~/.xauth")
        .output()
        .expect("Unable to type username");
    tracing::debug!("Typing username output: {:?}", out);

    // move to password
    let out = Command::new("xdotool")
        .args(["key", "--window", &window, "0xff09"])
        // .env("DISPLAY", ":0.0")
        .env("DISPLAY", ":99.0")
        .env("XAUTHORITY", "~/.xauth")
        .output()
        .expect("Unable to tab to password");
    tracing::debug!("Tab to password output: {:?}", out);

    let out = Command::new("xdotool")
        .args(["key", "--window", &window, "ctrl+a"])
        // .env("DISPLAY", ":0.0")
        .env("DISPLAY", ":99.0")
        .env("XAUTHORITY", "~/.xauth")
        .output()
        .expect("Unable to select password");
    tracing::debug!("Select password output: {:?}", out);

    let out = Command::new("xdotool")
        .args(["key", "--window", &window, "Delete"])
        // .env("DISPLAY", ":0.0")
        .env("DISPLAY", ":99.0")
        .env("XAUTHORITY", "~/.xauth")
        .output()
        .expect("Unable to delete password");
    tracing::debug!("Delete password output: {:?}", out);

    let out = Command::new("xdotool")
        .args(["type", "--window", &window, settings.password.expose_secret()])
        // .env("DISPLAY", ":0.0")
        .env("DISPLAY", ":99.0")
        .env("XAUTHORITY", "~/.xauth")
        .output()
        .expect("Unable to type password");
    tracing::debug!("Type password output: {:?}", out);

    let out = Command::new("xdotool")
        .args(["key", "--window", &window, "Return"])
        // .env("DISPLAY", ":0.0")
        .env("DISPLAY", ":99.0")
        .env("XAUTHORITY", "~/.xauth")
        .output()
        .expect("Unable to enter user credentials");
    tracing::debug!("Enter credentials output: {:?}", out);

    // wait for the characters to load
    tracing::info!("Waiting for character screen to load");
    thread::sleep(time::Duration::from_millis(settings.starsonatastartup.character_load_sleep));

    // first search for the star sonata window
    let output = Command::new("xdotool")
        .args(["search", "--name", ".*Star Sonata.*"])
        // .env("DISPLAY", ":0.0")
        .env("DISPLAY", ":99.0")
        .env("XAUTHORITY", "~/.xauth")
        .output()
        .expect("Unable to search for Star Sonata window.");
    println!("{:?}", &output.stdout);
    let window = String::from_utf8_lossy(&output.stdout).trim_end().to_string();
    tracing::info!("Found window id: {:?} for Star Sonata.", window);
    
    let out = Command::new("xdotool")
        .args(["key", "--window", &window, "Return"])
        // .env("DISPLAY", ":0.0")
        .env("DISPLAY", ":99.0")
        .env("XAUTHORITY", "~/.xauth")
        .output()
        .expect("Unable to select character");
    tracing::debug!("Character select output: {:?}", out);
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

    let (mut handle, window) = ss_start(enigo.clone(), settings.clone());

    ss_login(enigo.clone(), settings.clone(), window);

    while running.load(Ordering::SeqCst) {}

    handle.kill().unwrap();
}
