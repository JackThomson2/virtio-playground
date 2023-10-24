use std::{sync::atomic::{fence, Ordering::{Release, Acquire}}, fs::File, io::{Result, BufWriter, Write, Read}, ffi::c_int};

use crate::{epoll::Epoll, poller::PollableQueue};

use super::virtqueue::{VirtQueue, DescriptorCell};

pub struct DeviceDriver<const S: usize, P: PollableQueue + Clone> {
    queue: *mut VirtQueue<S>,

    available_index: u16,
    free_index: u16,

    file: Option<File>,

    poller: P,
}

impl <const S: usize> DeviceDriver<S, Epoll> {
    pub fn new_epoll(queue: *mut VirtQueue<S>, listen_fd: c_int, send_fs: c_int) -> Self {
        Self::new(queue, Epoll::new(listen_fd, send_fs))
    }
}

impl<const S: usize, P: PollableQueue + Clone> DeviceDriver<S, P> {

    pub fn new(queue: *mut VirtQueue<S>, poller: P) -> Self {
        Self {
            queue,
            available_index: 0,
            free_index: 0,

            file:  None,

            poller
        }
    }


    pub fn open_file(&mut self, file_name: &str) -> Result<()> {
        let file_opened = File::create(file_name)?;
        self.file = Some(file_opened);

        Ok(())
    }

    pub fn write_to_file(&mut self, contents: &str) -> Result<()> {
        if let Some(file) = self.file.as_mut() {
            let mut buf_writer = BufWriter::new(file);
            buf_writer.write_all(contents.as_bytes())?;
            buf_writer.flush()?;
        }

        Ok(())
    }

    pub fn read_to_slice(&mut self, buffer: &mut [u8], length: u64) -> Result<()> {
        if let Some(file) = self.file.as_ref() {
            let mut handle = file.take(length);

            handle.read(buffer)?;
        }

        Ok(())
    }

    pub fn close_file(&mut self) {
        self.file = None;
    }

    pub unsafe fn notify_poller(&mut self) {
        self.poller.submit_event();
    }

    pub unsafe fn wait_for_event(&mut self) {
        self.poller.wait_for_event()
    }

    pub unsafe fn poll_available_queue(&mut self) -> Option<(*mut DescriptorCell, u16)>{
        let queue = self.queue.as_mut().unwrap();
        let available_ring = queue.available.as_mut().unwrap();

        fence(Acquire);

        let ring_idx = available_ring.get_idx();

        if self.available_index == ring_idx {
            return None;
        }

        let loading_idx = self.available_index;
        let available_ring_pos = available_ring.get_ring_from_idx(loading_idx).read_volatile();

        self.available_index += 1;

        Some((queue.get_descriptor_from_idx(available_ring_pos), available_ring_pos))
    }

    pub unsafe fn submit_to_used_queue(&mut self, cell_pos: u16) {
        let queue = self.queue.as_mut().unwrap();
        let used_ring = queue.used.as_mut().unwrap();

        let ring_cell = used_ring.get_ring_from_idx(self.available_index).as_mut().unwrap();
        (&mut ring_cell.id as *mut u16).write_volatile(cell_pos);

        used_ring.increment_idx(S as u16);

        fence(Release);

        self.free_index += 1;
        self.free_index &= (S as u16) - 1;

        self.notify_poller();
    }
}

unsafe impl<const S: usize, P: PollableQueue + Clone> Send for DeviceDriver<S, P> {}
