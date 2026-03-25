use std::sync::Arc;
use std::process::{Command, Child, Stdio};
use std::{thread, time::Duration};

use secrecy::ExposeSecret;
use tracing::{instrument, info, warn, debug};

use process_wrap::std::*;

#[cfg(not(target_os = "linux"))]
use enigo::{Key, Keyboard, Enigo, Settings, Direction::{Click, Press, Release}};

use super::config::AppConfig;


#[macro_export]
macro_rules! xdotool {
    ([$($x:expr),*], $err_str:expr) => {
        {
            let mut c = Command::new("xdotool");
            c.env("DISPLAY", ":99.0");
            c.env("XAUTHORITY", "/home/ubuntu/.xauth");
            $(
                c.arg($x);
            )*
            c.output().expect($err_str)
        }
    };
    ([$($x:expr),*], $err_str:expr, env($e_var:expr, $e_val:expr)) => {
        {
            let mut c = Command::new("xdotool");
            c.env($e_var, $e_val);
            $(
                c.arg($x);
            )*
            c.output().expect($err_str)
        }
    }
}


#[cfg(not(target_os = "linux"))]
pub fn get_sleep_time(settings: Arc<AppConfig>) -> u64 {
    (settings.starsonata_startup.initial_sleep + settings.starsonata_startup.client_load_sleep) / 1000
}


#[cfg(target_os = "linux")]
pub fn get_sleep_time(settings: Arc<AppConfig>) -> u64 {
    (settings.starsonata_startup.client_load_sleep) / 1000
}


#[cfg(not(target_os = "linux"))]
#[instrument(skip(settings))]
pub fn starsonata_start(settings: Arc<AppConfig>) -> (Box<dyn ChildWrapper>, Option<String>) {
    // let handle = Command::new(&settings.starsonata_startup.ss_path)
    //     .spawn()
    //     .expect("Unable to start Star Sonata exe");
    let handle = CommandWrap::with_new(&settings.starsonata_startup.ss_path, |_| {})
        .wrap(JobObject)
        .spawn()
        .expect("Unable to start Star Sonata exe.");
    
    info!("Started exe, waiting {}s for client to get to initial options screen.", settings.starsonata_startup.initial_sleep / 1000);
    thread::sleep(Duration::from_millis(settings.starsonata_startup.initial_sleep));
    info!("Waited {}s, client should be on initial options screen.", settings.starsonata_startup.initial_sleep / 1000);

    let mut enigo = Enigo::new(&Settings::default()).expect("Unable to setup enigo");
    enigo.key(Key::Return, Click).expect("Unable to press Return");
    info!("Pressed Return, loading main client.");

    thread::sleep(Duration::from_millis(settings.starsonata_startup.client_load_sleep));
    info!("Waited {}s for the client to load, moving to login.", settings.starsonata_startup.client_load_sleep / 1000);

    return (handle, None);
}

#[cfg(target_os = "linux")]
#[tracing::instrument(skip(settings))]
pub fn starsonata_start(settings: Arc<AppConfig>) -> (Box<dyn ChildWrapper>, Option<String>) {
    // let handle = Command::new("xvfb-run")
    //     .args(["-f", "/home/ubuntu/.xauth", "-n", "99", "wine", &settings.starsonata_startup.ss_path])
    //     .spawn()
    //     .expect("Unable to start Star Sonata exe");
    let handle = CommandWrap::with_new("xvfb-run", |command| {command.args(["-f", "/home/ubuntu/.xauth", "-n", "99", "wine", &settings.starsonata_startup.ss_path]);})
        .wrap(ProcessGroup::leader())
        .stdout(Stdio::null())
        .spawn()
        .expect("Unable to start Star Sonata exe");
    
    info!("Started exe, waiting {}s for the client to load", settings.starsonata_startup.client_load_sleep / 1000);
    thread::sleep(Duration::from_millis(settings.starsonata_startup.client_load_sleep));

    // first search for the StarSonata window
    let output = xdotool!(["search", "--name", "Star Sonata$"], "Unable to search for the Star Sonata window.");
    let window = String::from_utf8_lossy(&output.stdout).trim_end().to_string();
    info!(
        "Waited {}s for the client to load, found window id: {:?} for Star Sonata. Proceeding to login",
        settings.starsonata_startup.client_load_sleep / 1000,
        window
    );

    return (handle, Some(window));
}

#[cfg(not(target_os = "linux"))]
#[instrument(skip(settings))]
pub fn starsonata_login(settings: Arc<AppConfig>, _: Option<String>) {
    let mut enigo = Enigo::new(&Settings::default()).expect("Unable to setup enigo");
    
    // should be on the login screen here with cursor selecting the username field
    // first, selecting existing username, remove, and retype just for safety
    info!("Setting username");
    enigo.key(Key::Control, Press).unwrap();
    enigo.key(Key::Unicode('a'), Click).unwrap();
    enigo.key(Key::Control, Release).unwrap();
    enigo.key(Key::Delete, Click).unwrap();
    enigo.text(&settings.starsonata_account.username).unwrap();

    // move onto password
    info!("Setting password");
    enigo.key(Key::Tab, Click).unwrap();
    enigo.key(Key::Control, Press).unwrap();
    enigo.key(Key::Unicode('a'), Click).unwrap();
    enigo.key(Key::Control, Release).unwrap();
    enigo.key(Key::Delete, Click).unwrap();
    enigo.text(settings.starsonata_account.password.expose_secret()).unwrap();

    // login
    enigo.key(Key::Return, Click).unwrap();

    info!("Waiting {}s for character screen to load.", settings.starsonata_startup.character_load_sleep / 1000);
    thread::sleep(Duration::from_millis(settings.starsonata_startup.character_load_sleep));

    info!("Selecting the first character via Return key.");
    enigo.key(Key::Return, Click).unwrap();
}


#[cfg(target_os = "linux")]
#[tracing::instrument(skip(settings))]
pub fn starsonata_login(settings: Arc<AppConfig>, window: Option<String>) {
    let window = window.expect("Window ID was not set");

    // Should be on the first login screen here. Cursor should be selecting the username field
    // Select the username and re-type
    info!("Selecting username for replacement");
    let out = xdotool!(["key", "--window", &window, "ctrl+a"], "Unable to select username");
    debug!("Select username output: {:?}", out);

    // ignoring delete key command here since selecting and then retyping should clear
    info!("Typing username");
    let out = xdotool!(["type", "--window", &window, &settings.starsonata_account.username], "Unable to type username");
    debug!("Typing username output: {:?}", out);

    // tab to password
    let out = xdotool!(["key", "--window", &window, "0xff09"], "Unable to tab to password");
    debug!("Tab to password output: {:?}", out);

    info!("Tabbed to password, selecting.");
    let out = xdotool!(["key", "--window", &window, "ctrl+a"], "Unable to select password");
    debug!("Select password output: {:?}", out);

    info!("Typing passsword");
    let out = xdotool!(["type", "--window", &window, settings.starsonata_account.password.expose_secret()], "Unable to type password");
    debug!("Type password output: {:?}", out);

    // log in
    info!("Logging in.");
    let out = xdotool!(["key", "--window", &window, "Return"], "Unable to press return to login");
    debug!("Press return output: {:?}", out);

    // wait for the character screen to load
    info!("Waiting {}s for character screen to load", settings.starsonata_startup.character_load_sleep / 1000);
    thread::sleep(Duration::from_millis(settings.starsonata_startup.character_load_sleep));

    info!("Selecting first character through pressing Return.");
    let out = xdotool!(["key", "--window", &window, "Return"], "Unable to press return to select character");
    debug!("Character select Return output: {:?}", out);
}