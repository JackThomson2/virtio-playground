#![feature(new_uninit)]

mod faux_blk;
mod comms;
mod terminal_thread;
mod device_thread;
mod os_thread;
mod async_driver;
mod virtio;

use std::{error::Error, thread};

use comms::CommsLink;

use device_thread::create_device_thread;
use terminal_thread::create_terminal;
use os_thread::create_os_thread;
use virtio::create_queue;

fn main() -> Result<(), Box<dyn Error>> {
    let (ui_comms, os_comms) = CommsLink::new_pair();
    let driver_queue = os_comms.tx.clone();

    let (host_driver, device_driver) = create_queue::<64>();

    let _os_thread = thread::spawn(move || {
        create_os_thread(os_comms, host_driver);
    });

    let ui_thread = thread::spawn(|| {
        create_terminal(ui_comms).unwrap();
    });

    let _driver_thread = thread::spawn(move || unsafe {
        create_device_thread(driver_queue, device_driver);
    });

    ui_thread.join().unwrap();

    Ok(())
}
