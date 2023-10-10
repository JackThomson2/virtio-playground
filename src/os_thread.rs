// A fake OS thread this will act as a virtual os to handle the file writes and interacting with
// the virtio thread

use std::{future::Future, pin::{Pin, pin}, time::Duration, task::Poll};

use tokio::runtime;

use crate::{comms::{CommsLink, Messages}, virtio::{guest_driver::GuestDriver, virtqueue::DescriptorCell}};

#[pin_project::pin_project]
struct DriverPoller<'a, const S: usize> {
    driver: &'a mut GuestDriver<S>
}

impl <'a, const S: usize> Future for DriverPoller<'a, S> {
    type Output = *mut DescriptorCell;

    fn poll(self: Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Self::Output> {
        let this = self.project();

        let mut delay_timer = delay();
        let pin_fut = pin!(delay_timer);

        let _ = pin_fut.poll(cx);

        unsafe {
            if let Some(found) = this.driver.check_avail_queue() {
                Poll::Ready(found)
            } else {
                Poll::Pending
            }
        }

    }
}

async fn delay() -> () {
    tokio::time::sleep(Duration::from_millis(500)).await
}


pub fn create_os_thread<const S: usize>(mut ui_comms: CommsLink, mut driver: GuestDriver<S>) {
    let rt = runtime::Builder::new_current_thread().enable_all().build().unwrap();

    let poller = DriverPoller { driver: &mut driver };

    rt.block_on(async {
        let start_message = Messages::String(format!("The os thread has booted!"));
        ui_comms.tx.send(start_message).await.unwrap();

        while let Some(res) = ui_comms.rx.recv().await {
            let ack_message = Messages::String(format!("The os thread acknowledged the message"));
            ui_comms.tx.send(ack_message).await.unwrap();

            if let Messages::FileWrite(file_name, file_contents) = res {
                let write_message = Messages::String(format!("Creating file with name {file_name} with contents: {file_contents}"));
                ui_comms.tx.send(write_message).await.unwrap();
            }
        }
    });
}
