use std::process::Command;
use std::{thread, time};

use enigo::{Key, Keyboard, Enigo, Settings};
use enigo::Direction::Click;

fn main() {
    let mut handle = Command::new(r"C:\Users\lukas\AppData\Roaming\Star Sonata 2 (Beta Client)\Star Sonata.exe").spawn().expect("Unable to start exe");

    thread::sleep(time::Duration::from_millis(3000));

    println!("done sleeping");

    let mut enigo = Enigo::new(&Settings::default()).unwrap();
    let _ = enigo.key(Key::Return, Click);

    // thread::sleep(time::Duration::from_millis(15000));
    // match handle.kill() {
    //     Ok(a) => println!("{:?}", a),
    //     Err(a) => println!("{:?}", a),
    // }
}
