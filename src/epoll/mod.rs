use libc::{c_int, epoll_event, EPOLLIN, epoll_create1, epoll_ctl, EPOLL_CTL_ADD, epoll_wait, c_void};
use crate::{comms::GLOBAL_COMMS, poller::PollableQueue};

#[derive(Copy, Clone, Debug)]
pub struct Epoll {
    listener_fd: c_int,
    publish_fd: c_int,

    event_fd: c_int,
}

impl Epoll {
    pub fn new(listener_fd: c_int, publish_fd: c_int) -> Self {
        let event_fd = unsafe { register_epoll_listener(listener_fd) };

        Self {
            listener_fd,
            publish_fd,

            event_fd
        }
    }
}

impl PollableQueue for Epoll {
    fn wait_for_event(&self) {
        unsafe { wait_for_epoll_event(self.event_fd, self.listener_fd) }
    }

    fn submit_event(&self) {
        unsafe { notify_epoll_fd(self.publish_fd) }
    }
}

unsafe impl Send for Epoll {}
unsafe impl Sync for Epoll {}

pub unsafe fn register_epoll_listener(incoming: c_int) -> c_int {
    let epoll_fd = unsafe {
        epoll_create1(0)
    };

    let mut event = epoll_event {
        events: EPOLLIN as u32,
        u64: 0
    };

    unsafe {
        epoll_ctl(epoll_fd, EPOLL_CTL_ADD, incoming, &mut event);
    }


    epoll_fd
}

pub unsafe fn wait_for_epoll_event(listen_fd: c_int, root_fd: c_int) {
    let mut events = [epoll_event { events: 0, u64: 0}; 10];

    loop {
        GLOBAL_COMMS.write_message(format!("Epoll waiting for fd: {listen_fd}"));
        let n = epoll_wait(listen_fd, events.as_mut_ptr(), events.len() as i32, -1);
        GLOBAL_COMMS.write_message(format!("Epoll got event for fd: {listen_fd}"));

        if n > 0 {
            read_buffer(root_fd);
            GLOBAL_COMMS.write_message(format!("Epoll read events for fd: {listen_fd}"));
            return;
        }
    }
}

pub unsafe fn read_buffer(listen_fd: c_int) {
    let mut buffer: [u8; 64] = [0; 64];

    loop {
        let bytes_read = libc::read(listen_fd, buffer.as_mut_ptr() as *mut libc::c_void, buffer.len());

        if bytes_read <= 0 {
            return;
        }
    }
}

pub unsafe fn notify_epoll_fd(notify_fd: c_int) {
    let data: [u8; 1] = [0];

    libc::write(notify_fd, data.as_ptr() as * const c_void, 1);

}
