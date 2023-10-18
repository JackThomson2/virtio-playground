use std::{time::Duration, thread};

use tokio::sync::mpsc::Sender;

use crate::{comms::Messages, virtio::{device_driver::DeviceDriver, virtqueue::DescriptorCell}, faux_blk};

unsafe fn read_string_from_cell(cell: &DescriptorCell) -> String {
    String::from_raw_parts(cell.addr as *mut u8, cell.length as usize, cell.length as usize)
}

unsafe fn read_message<const S: usize>(comms: &Sender<Messages>, driver: &mut DeviceDriver<S>, cell: &DescriptorCell) {
    if cell.flags & faux_blk::FILE_OPEN_FLAG > 0{
        let file_name = read_string_from_cell(cell);

        let result = driver.open_file(&file_name);

        let message = format!("Submitted file open it was success: {}", result.is_ok());
        comms.blocking_send(Messages::DriverMessage(message));
    } else if cell.flags & faux_blk::FILE_WRITE_CONTENTS_FLAG > 0 {
        let file_contents = read_string_from_cell(cell);

        let result = driver.write_to_file(&file_contents);

        let message = format!("Submitted file write it was success: {}", result.is_ok());
        comms.blocking_send(Messages::DriverMessage(message));
    } else if cell.flags & faux_blk::FILE_CLOSE_FLAG > 0 {
        driver.close_file();

        let message = format!("Submitted file close");
        comms.blocking_send(Messages::DriverMessage(message));
    } else {
        let message = format!("Unknown flag of {}", cell.flags);
        comms.blocking_send(Messages::DriverMessage(message));
    }
}

pub unsafe fn create_device_thread<const S: usize>(ui_comms: Sender<Messages>, mut driver: DeviceDriver<S>) {
    ui_comms.blocking_send(Messages::DriverMessage("Hardware device booted!".to_string())).unwrap();

    loop {
        thread::sleep(Duration::from_millis(500));

        if let Some((cell, idx)) = driver.poll_available_queue() {
            let found_data = cell.as_ref().unwrap();

            read_message(&ui_comms, &mut driver, found_data)
        }
    }
}
