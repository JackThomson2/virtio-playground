use std::{sync::atomic::{fence, Ordering::{Release, Acquire}}, fs::File, io::{Result, BufWriter, Write}};

use super::virtqueue::{VirtQueue, DescriptorCell};

pub struct DeviceDriver<const S: usize> {
    queue: *mut VirtQueue<S>,

    available_index: u16,
    free_index: u16,

    file: Option<File>,
}

impl<const S: usize> DeviceDriver<S> {

    pub fn new_driver(queue: *mut VirtQueue<S>) -> Self {
        Self {
            queue,
            available_index: 0,
            free_index: 0,

            file: None,
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

    pub fn close_file(&mut self) {
        self.file = None;
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
    }
}

unsafe impl<const S: usize> Send for DeviceDriver<S> {}
