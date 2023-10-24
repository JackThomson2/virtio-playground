use std::{time::Duration, thread, mem::ManuallyDrop, slice};

use tokio::sync::mpsc::Sender;

use crate::{comms::Messages, virtio::{device_driver::DeviceDriver, virtqueue::DescriptorCell}, faux_blk::{self, FILE_STATE_FLAG, STATE_SUCCESS, FILE_READ}};

unsafe fn read_string_from_cell(cell: &DescriptorCell) -> ManuallyDrop<String> {
    ManuallyDrop::new(String::from_raw_parts(cell.addr as *mut u8, cell.length as usize, cell.length as usize))
}

unsafe fn write_file_to_cell<const S: usize>(driver: &mut DeviceDriver<S>, cell: &mut DescriptorCell) {
    let data_slice = slice::from_raw_parts_mut(cell.addr as *mut u8, cell.length as usize);

    driver.read_to_slice(data_slice, cell.length as u64).unwrap()
}

unsafe fn write_success_to_cell(cell: &mut DescriptorCell) {
    (&mut cell.flags as *mut u16).write_volatile(FILE_STATE_FLAG | STATE_SUCCESS);
}

unsafe fn read_message<const S: usize>(comms: &Sender<Messages>, driver: &mut DeviceDriver<S>, cell: &mut DescriptorCell, idx: u16) {
    if cell.flags & faux_blk::FILE_OPEN_FLAG > 0{
        let file_name = read_string_from_cell(cell);

        let result = driver.open_file(&file_name);

        let message = format!("Submitted file open it was success: {}", result.is_ok());
        comms.blocking_send(Messages::DriverMessage(message)).unwrap();

        write_success_to_cell(cell);
    } else if cell.flags & faux_blk::FILE_WRITE_CONTENTS_FLAG > 0 {
        if cell.flags & faux_blk::FILE_WRITE > 0 {
            let file_contents = read_string_from_cell(cell);

            let result = driver.write_to_file(&file_contents);

            let message = format!("Submitted file write it was success: {}", result.is_ok());
            comms.blocking_send(Messages::DriverMessage(message)).unwrap();

            write_success_to_cell(cell);
        } else if cell.flags & faux_blk::FILE_READ > 0 {
            write_file_to_cell(driver, cell);

            let message = format!("Recieved read request");
            comms.blocking_send(Messages::DriverMessage(message)).unwrap();

            (&mut cell.flags as *mut u16).write_volatile(FILE_READ | STATE_SUCCESS);
        }
    } else if cell.flags & faux_blk::FILE_CLOSE_FLAG > 0 {
        driver.close_file();

        let message = format!("Submitted file close");
        comms.blocking_send(Messages::DriverMessage(message)).unwrap();

        write_success_to_cell(cell);
    } else {
        let message = format!("Unknown flag of {}", cell.flags);
        comms.blocking_send(Messages::DriverMessage(message)).unwrap();
    }

    driver.submit_to_used_queue(idx);
}

pub unsafe fn create_device_thread<const S: usize>(ui_comms: Sender<Messages>, mut driver: DeviceDriver<S>) {
    ui_comms.blocking_send(Messages::DriverMessage("Hardware device booted!".to_string())).unwrap();

    loop {
        while let Some((cell, idx)) = driver.poll_available_queue() {
            let found_data = cell.as_mut().unwrap();

            read_message(&ui_comms, &mut driver, found_data, idx)
        }

        ui_comms.blocking_send(Messages::DriverMessage("Waiting for epoll event".to_string())).unwrap();
        driver.wait_for_epoll();
        ui_comms.blocking_send(Messages::DriverMessage("Epoll event Recieved".to_string())).unwrap();
    }
}
