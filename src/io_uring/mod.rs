use std::sync::Arc;

use io_uring::IoUring as Ring;
use io_uring::{opcode, types};

use libc::{pipe2, O_NONBLOCK, c_void};

use crate::comms::GLOBAL_COMMS;
use crate::epoll::read_buffer;
use crate::poller::PollableQueue;

#[derive(Clone)]
pub struct IOUring {
    entries: u32,

    send_ring_ref: Arc<Ring>,
    recv_ring_ref: Arc<Ring>,

    publish_fd: i32,
    listen_fd: i32,
}

pub fn create_rings(size: u32) -> (IOUring, IOUring) {
    let mut guest_to_device = [-1; 2];
    let mut device_to_guest = [-1; 2];

    unsafe {
        pipe2(guest_to_device.as_mut_ptr(), O_NONBLOCK);
        pipe2(device_to_guest.as_mut_ptr(), O_NONBLOCK);
    }

    let ring_one = Arc::new(Ring::new(size).unwrap());
    let ring_two = Arc::new(Ring::new(size).unwrap());

    let device_poller = IOUring::new(size, ring_one.clone(), ring_two.clone(), guest_to_device[0], device_to_guest[1]);
    let guest_poller = IOUring::new(size, ring_two.clone(), ring_one.clone(), device_to_guest[0], guest_to_device[1]);

    (device_poller, guest_poller)
}

impl IOUring {
    pub fn new(entries: u32, send_ring_ref: Arc<Ring>, recv_ring_ref: Arc<Ring>, submission_fd: i32, completion_fd: i32) -> Self {
        Self {
            entries,
            send_ring_ref,
            recv_ring_ref,
            listen_fd: submission_fd,
            publish_fd: completion_fd
        }
    }

    unsafe fn poll_and_wait(&self) {
        GLOBAL_COMMS.write_message(format!("IO_Uring building poll message for fd: {}", self.listen_fd));
        let read_e = opcode::PollAdd::new(types::Fd(self.listen_fd), libc::POLLIN as _)
            .build()
            .user_data(self.listen_fd as _);

        self.recv_ring_ref.submission_shared().push(&read_e).unwrap();
        self.recv_ring_ref.submit_and_wait(1).unwrap();

        read_buffer(self.listen_fd);

        self.recv_ring_ref.completion_shared().next().expect("completion queue is empty");
    }

    unsafe fn write_to_fd(&self) {
        let data: [u8; 1] = [0];
        libc::write(self.publish_fd, data.as_ptr() as * const c_void, 1);
    }
}

impl PollableQueue for IOUring {

    fn wait_for_event(&self) {
        unsafe { self.poll_and_wait(); }
    }

    fn submit_event(&self) {
        unsafe { self.write_to_fd(); }
    }

}
