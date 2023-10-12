use std::{time::Duration, thread};

use tokio::sync::mpsc::Sender;

use crate::{comms::Messages, virtio::device_driver::DeviceDriver};

pub unsafe fn create_device_thread<const S: usize>(ui_comms: Sender<Messages>, mut driver: DeviceDriver<S>) {
    ui_comms.blocking_send(Messages::DriverMessage("Hardware device booted!".to_string())).unwrap();

    loop {
        thread::sleep(Duration::from_millis(500));

        if let Some((cell, idx)) = driver.poll_available_queue() {
            let found_data = cell.as_ref().unwrap();

            let string_read = String::from_raw_parts(found_data.addr as *mut u8, found_data.length as usize, found_data.length as usize);

            let message = format!("Got a message pointing to idx: {idx}. It has length of {}", found_data.length);
            ui_comms.blocking_send(Messages::DriverMessage(message)).unwrap();

            let message = format!("Parsed message is: {}", string_read);
            ui_comms.blocking_send(Messages::DriverMessage(message)).unwrap();
        }
    }
}
