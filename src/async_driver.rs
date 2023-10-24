use std::{pin::{Pin, pin}, task::{Poll, Waker}, sync::{Arc, Mutex}, thread};
use pin_project::pinned_drop;
use tokio::time::Instant;
use tokio_stream::Stream;

use crate::{virtio::{guest_driver::GuestDriver, virtqueue::DescriptorCell}, poller::PollableQueue};

pub struct SharedState {
    complete: bool,
    waker: Option<Waker>
}

#[pin_project::pin_project(PinnedDrop)]
pub struct DriverPoller<'a, const S: usize, P: PollableQueue + Copy + Clone + Send> {
    driver: &'a mut GuestDriver<S, P>,
    last_update: Instant,
    shared_state: Arc<Mutex<SharedState>>
}

impl <'a, const S: usize, P: PollableQueue + Copy + Clone + Send + 'static> DriverPoller<'a, S, P> {
    pub fn new(driver: &'a mut GuestDriver<S, P>) -> Self {
        Self {
            driver,
            last_update: Instant::now(),
            shared_state : Arc::new(Mutex::new(SharedState {
                complete: false,
                waker: None
            })),
        }
    }

    pub unsafe fn get_driver(&self) -> *mut GuestDriver<S, P> {
        let const_ptr = self.driver as *const GuestDriver<S, P>;
        const_ptr as *mut GuestDriver<S, P>
    }

    pub unsafe fn get_driver_ref(&mut self) ->&mut GuestDriver<S, P> {
        self.driver
    }

    pub fn delayed_poller(&self) -> () {
        let shared_state = self.shared_state.clone();
        let poll_item = self.driver.poll_interface.clone();

        thread::spawn(move || {
            loop {
                let mut state = shared_state.lock().unwrap();

                if state.complete {
                    return
                }

                if let Some(waker) = state.waker.take() {
                    waker.wake();
                }

                drop(state);

                poll_item.wait_for_event();
            }
        });
    }
}

impl <'a, const S: usize, P: PollableQueue + Copy + Clone + Send> Stream for DriverPoller<'a, S, P> {
    type Item = (*mut DescriptorCell, u16);

    fn poll_next(self: Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.project();

        unsafe {
            if let Some(found) = this.driver.check_used_queue() {
                Poll::Ready(Some(found))
            } else {
                let mut state = this.shared_state.lock().unwrap();
                state.waker = Some(cx.waker().clone());

                Poll::Pending
            }
        }
    }
}


#[pinned_drop]
impl <'a, const S: usize, P: PollableQueue + Copy + Clone + Send> PinnedDrop for DriverPoller<'a, S, P> {
    fn drop(self: Pin<&mut Self>) {
        let mut state = self.shared_state.lock().unwrap();
        state.complete = true;
    }
}

