use std::process::{Command, Child};
use std::{thread, time};
// use std::sync::{Arc, Mutex};
use std::rc::Rc;
use std::cell::RefCell;

use enigo::{Key, Keyboard, Enigo, Settings};
use enigo::Direction::{Click, Press, Release};

use tracing;
use tracing_subscriber;

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
fn ss_login(mut enigo: Rc<RefCell<Enigo>>) {
    let mut enigo = enigo.borrow_mut();
    // Should be on the login screen here with cursor selecting the username field
    // First, select existing username, remove and then retype
    let _ = enigo.key(Key::Control, Press);
    let _ = enigo.key(Key::Unicode('a'), Click);
    let _ = enigo.key(Key::Control, Release);
    let _ = enigo.key(Key::Delete, Click);
}


fn main() {
    let subscriber = tracing_subscriber::fmt()
        .with_max_level(tracing::Level::TRACE)
        .init();
    
    // let enigo = Arc::new(Mutex::new(Enigo::new(&Settings::default()).unwrap()));
    let enigo = Rc::new(RefCell::new(Enigo::new(&Settings::default()).unwrap()));

    let mut handle = ss_start(enigo.clone());

    ss_login(enigo.clone());

    // thread::sleep(time::Duration::from_millis(15000));
    // match handle.kill() {
    //     Ok(a) => println!("{:?}", a),
    //     Err(a) => println!("{:?}", a),
    // }
}
