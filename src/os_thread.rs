// A fake OS thread this will act as a virtual os to handle the file writes and interacting with
// the virtio thread

use std::mem::ManuallyDrop;

use tokio_stream::StreamExt;
use futures::FutureExt;

use tokio::runtime;

use crate::faux_blk;
use crate::async_driver::DriverPoller;

use crate::virtio::virtqueue::DescriptorCell;
use crate::{comms::{CommsLink, Messages}, virtio::guest_driver::GuestDriver};

unsafe fn write_string_to_queue<const S: usize>(driver: &mut GuestDriver<S>, message: &str, flag: u16) -> bool {
    let (cell_ptr, idx) = match driver.get_descriptor_cell() {
        Some(res) => res,
        None => return false,
    };

    let cell = cell_ptr.as_mut().unwrap();
    let mut message = message.to_string();
    message.shrink_to_fit();

    let device_address = ManuallyDrop::new(message);
    let length = device_address.len();

    cell.addr = device_address.as_ptr() as u64;
    cell.length = length as u32;
    cell.flags = flag;
    cell.next = 0;

    driver.submit_to_avail_queue(idx);

    return true
}

unsafe fn write_file_contents<const S: usize>(driver_ptr: *mut GuestDriver<S>, file_name: &str, file_contents: &str) -> bool {
    let driver = driver_ptr.as_mut().unwrap();

    const OPEN_FILE_FLAG: u16= faux_blk::FILE_WRITE | faux_blk::FILE_OPEN_FLAG;
    if !write_string_to_queue(driver, file_name, OPEN_FILE_FLAG) {
        return false;
    }

    const WRITE_CONTENTS: u16 = faux_blk::FILE_WRITE | faux_blk::FILE_WRITE_CONTENTS_FLAG;
    if !write_string_to_queue(driver, file_contents, WRITE_CONTENTS) {
        return false;
    }

    const CLOSE_FILE_FLAG: u16 = faux_blk::FILE_WRITE | faux_blk::FILE_CLOSE_FLAG;
    if !write_string_to_queue(driver, "", CLOSE_FILE_FLAG) {
        return false;
    }

    true
}

pub fn create_os_thread<const S: usize>(mut ui_comms: CommsLink, mut driver: GuestDriver<S>) {
    let rt = runtime::Builder::new_current_thread().enable_all().build().unwrap();

    let mut poller = DriverPoller::new(&mut driver);
    let driver_ptr = unsafe { poller.get_driver() };

    poller.delayed_poller();

    rt.block_on(async {
        let start_message = Messages::OSMessage(format!("The os thread has booted!"));
        ui_comms.tx.send(start_message).await.unwrap();

        loop {
            let ui_comms_link = ui_comms.rx.recv().fuse();
            let poller_loop = poller.next().fuse();

            tokio::select! {
                Some(res) = ui_comms_link => {
                    let ack_message = Messages::OSMessage(format!("The os thread acknowledged the message"));
                    ui_comms.tx.send(ack_message).await.unwrap();

                    if let Messages::FileWrite(file_name, file_contents) = res {
                        let result = unsafe { write_file_contents(driver_ptr, &file_name, &file_contents) };
                        let write_message = Messages::OSMessage(format!("Writing to the driver was successful: {result}"));

                        ui_comms.tx.send(write_message).await.unwrap();
                    }
                },
                Some((cell, idx)) = poller_loop => {
                    unsafe { poller.get_driver_ref().release_back_to_pool(cell, idx) }
                    ui_comms.tx.send(Messages::OSMessage(format!("We got a notification from our device driver!"))).await.unwrap();
                }
            }
        }
    });
}
