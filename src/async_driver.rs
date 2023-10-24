use std::{pin::{Pin, pin}, time::Duration, task::{Poll, Waker}, ptr::null_mut, sync::{Arc, Mutex, atomic::{AtomicBool, Ordering::Relaxed}}, thread};
use pin_project::pinned_drop;
use tokio::time::Instant;
use tokio_stream::Stream;

use crate::{virtio::{guest_driver::GuestDriver, virtqueue::DescriptorCell}, epoll::wait_for_epoll_event};

pub struct SharedState {
    complete: bool,
    waker: Option<Waker>
}

#[pin_project::pin_project(PinnedDrop)]
pub struct DriverPoller<'a, const S: usize> {
    driver: &'a mut GuestDriver<S>,
    last_update: Instant,
    shared_state: Arc<Mutex<SharedState>>
}

impl <'a, const S: usize> DriverPoller<'a, S> {
    pub fn new(driver: &'a mut GuestDriver<S>) -> Self {
        Self {
            driver,
            last_update: Instant::now(),
            shared_state : Arc::new(Mutex::new(SharedState {
                complete: false,
                waker: None
            })),
        }
    }

    pub unsafe fn get_driver(&self) -> *mut GuestDriver<S> {
        let const_ptr = self.driver as *const GuestDriver<S>;
        const_ptr as *mut GuestDriver<S>
    }

    pub unsafe fn get_driver_ref(&mut self) ->&mut GuestDriver<S> {
        self.driver
    }

    pub fn delayed_poller(&self) -> () {
        let shared_state = self.shared_state.clone();
        let epoll_fd = self.driver.epoll_listener;
        let epoll_root_fd = self.driver.epoll_listener_fd;

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

                unsafe {
                    wait_for_epoll_event(epoll_fd, epoll_root_fd)
                }
            }
        });
    }
}

impl <'a, const S: usize> Stream for DriverPoller<'a, S> {
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
impl <'a, const S: usize> PinnedDrop for DriverPoller<'a, S> {
    fn drop(self: Pin<&mut Self>) {
        let mut state = self.shared_state.lock().unwrap();
        state.complete = true;
    }
}

