use std::process::{Command, Child};
use std::{thread, time};
// use std::sync::{Arc, Mutex};
use std::rc::Rc;
use std::cell::RefCell;

use config::Config;
use enigo::{Key, Keyboard, Enigo, Settings};
use enigo::Direction::{Click, Press, Release};
use tracing;
use tracing_subscriber;
use secrecy::{SecretBox, ExposeSecret};


#[derive(serde::Deserialize, Debug)]
struct AppConfig {
    username: String,
    password: SecretBox<String>,
}

#[tracing::instrument]
fn ss_start(mut enigo: Rc<RefCell<Enigo>>) -> Child {
    let mut handle = Command::new(r"C:\Users\lukas\AppData\Roaming\Star Sonata 2 (Beta Client)\Star Sonata.exe").spawn().expect("Unable to start exe");
    thread::sleep(time::Duration::from_millis(3000));

    tracing::info!("waited 3s starting SS client from options menu screen.");
    let mut enigo = enigo.borrow_mut();
    let _ = enigo.key(Key::Return, Click);

    // wait for the client to load
    thread::sleep(time::Duration::from_millis(10000));

    return handle;
}

#[tracing::instrument]
fn ss_login(mut enigo: Rc<RefCell<Enigo>>, settings: Rc<AppConfig>) {
    let mut enigo = enigo.borrow_mut();
    // Should be on the login screen here with cursor selecting the username field
    // First, select existing username, remove and then retype
    let _ = enigo.key(Key::Control, Press);
    let _ = enigo.key(Key::Unicode('a'), Click);
    let _ = enigo.key(Key::Control, Release);
    let _ = enigo.key(Key::Delete, Click);

    // enter the username
    // for ch in (&settings.username).chars() {
    //     let _ = enigo.key(Key::Unicode(ch), Click);
    // }
    enigo.text(&settings.username).unwrap();
    // move onto password
    let _ = enigo.key(Key::Tab, Click);
    let _ = enigo.key(Key::Control, Press);
    let _ = enigo.key(Key::Unicode('a'), Click);
    let _ = enigo.key(Key::Control, Release);
    let _ = enigo.key(Key::Delete, Click);

    // for ch in (&settings.password).chars() {
    //     let _ = enigo.key(Key::Unicode(ch), Click);
    // }
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

    let subscriber = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .init();
    
    // let enigo = Arc::new(Mutex::new(Enigo::new(&Settings::default()).unwrap()));
    let enigo = Rc::new(RefCell::new(Enigo::new(&Settings::default()).unwrap()));

    let mut handle = ss_start(enigo.clone());

    ss_login(enigo.clone(), settings.clone());

    // thread::sleep(time::Duration::from_millis(15000));
    // match handle.kill() {
    //     Ok(a) => println!("{:?}", a),
    //     Err(a) => println!("{:?}", a),
    // }
}
