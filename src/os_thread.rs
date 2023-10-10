// A fake OS thread this will act as a virtual os to handle the file writes and interacting with
// the virtio thread

use tokio_stream::StreamExt;
use futures::FutureExt;

use tokio::runtime;

use crate::async_driver::DriverPoller;

use crate::{comms::{CommsLink, Messages}, virtio::guest_driver::GuestDriver};

pub fn create_os_thread<const S: usize>(mut ui_comms: CommsLink, mut driver: GuestDriver<S>) {
    let rt = runtime::Builder::new_current_thread().enable_all().build().unwrap();

    let mut poller = DriverPoller::new(&mut driver);
    poller.delayed_poller();

    rt.block_on(async {
        let start_message = Messages::String(format!("The os thread has booted!"));
        ui_comms.tx.send(start_message).await.unwrap();

        loop {
            let ui_comms_link = ui_comms.rx.recv().fuse();
            let poller_loop = poller.next().fuse();

            tokio::select! {
                Some(res) = ui_comms_link => {
                    let ack_message = Messages::String(format!("The os thread acknowledged the message"));
                    ui_comms.tx.send(ack_message).await.unwrap();

                    if let Messages::FileWrite(file_name, file_contents) = res {
                        let write_message = Messages::String(format!("Creating file with name {file_name} with contents: {file_contents}"));
                        ui_comms.tx.send(write_message).await.unwrap();
                    }
                },
                Some(poll) = poller_loop => {
                    ui_comms.tx.send(Messages::String(format!("We got a notification from our device driver!"))).await.unwrap();
                }
            }
        }
    });
}
