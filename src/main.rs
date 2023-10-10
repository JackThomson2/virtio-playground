#![feature(new_uninit)]

mod comms;
mod terminal_thread;
mod os_thread;
mod virtio;

use std::{error::Error, thread};

use comms::CommsLink;

use terminal_thread::create_terminal;
use os_thread::create_os_thread;
use virtio::create_queue;

fn main() -> Result<(), Box<dyn Error>> {
    let (ui_comms, os_comms) = CommsLink::new_pair();

    let drivers = create_queue::<100>();

    let _os_thread = thread::spawn(move || {
        create_os_thread(os_comms, drivers);
    });

    let ui_thread = thread::spawn(|| {
        create_terminal(ui_comms).unwrap();
    });

    ui_thread.join().unwrap();

    Ok(())
}
