use std::sync::RwLock;
use std::sync::mpsc;
use std::thread;

static FLAG: f32 = pi(true);

const fn pi(high_precision: bool) -> f32 {
    if high_precision {
        3.1415926
    } else {
        3.14
    }
}

fn main1() {
    const PI: f32 = pi(true);
    let rw = RwLock::new(1);
    let rg = rw.read().unwrap();
    let wg = rw.write().unwrap();
}

fn main() {
    let (tx, rx) = mpsc::channel();

    thread::spawn(move || {
        let val = 1;
        tx.send(val).unwrap();
    });

    let received = rx.recv().unwrap();
    println!("Got: {}", received);
}