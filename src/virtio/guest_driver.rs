use std::sync::atomic::{fence, Ordering::{Release, Acquire}};

use super::virtqueue::{VirtQueue, DescriptorCell};

pub struct GuestDriver<const S: usize> {
    queue: *mut VirtQueue<S>,

    available_index: u16,
    free_index: u16,

    descriptor_item_index: usize,
    free_descriptor_cells: [u16; S],
}

impl<const S: usize> GuestDriver<S> {

    pub fn new_driver(queue: *mut VirtQueue<S>) -> Self {
        let mut free_cells = [0; S];

        for (idx, cell) in free_cells.iter_mut().enumerate() {
            *cell = idx as u16;
        }

        Self {
            queue,
            available_index: 0,
            free_index: 0,

            descriptor_item_index: S,
            free_descriptor_cells: free_cells
        }
    }

    pub unsafe fn get_descriptor_cell(&mut self) -> Option<(*mut DescriptorCell, u16)> {
        if self.descriptor_item_index == 0 {
            return None;
        }

        self.descriptor_item_index -= 1;

        let queue = self.queue.as_mut().unwrap();
        let desc_cell_idx = self.free_descriptor_cells[self.descriptor_item_index];

        Some((queue.get_descriptor_from_idx(desc_cell_idx), desc_cell_idx))
    }

    pub unsafe fn submit_to_avail_queue(&mut self, idx: u16) {
        let queue = self.queue.as_mut().unwrap();
        let available_ring = queue.available.as_mut().unwrap();

        let ring_cell = available_ring.get_ring_from_idx(self.available_index);
        *ring_cell = idx;

        available_ring.increment_idx(S as u16);

        fence(Release);

        self.available_index += 1;
        self.available_index &= (S as u16) - 1;
    }

    pub unsafe fn check_used_queue(&mut self) -> Option<(*mut DescriptorCell, u16)> {
        let queue = self.queue.as_mut().unwrap();
        let used = queue.used.as_mut().unwrap();

        let current_idx = used.get_idx();

        // If this happens there have been no updates
        if current_idx == self.free_index {
            return None;
        }

        let freed_item = used.get_ring_from_idx(self.free_index).as_ref().unwrap();
        self.free_index += 1;

        Some((queue.get_descriptor_from_idx(freed_item.id) as *mut DescriptorCell, freed_item.id))
    }

    pub unsafe fn release_back_to_pool(&mut self, cell: *mut DescriptorCell, idx: u16) {
        self.free_descriptor_cells[self.descriptor_item_index + 1] = idx;
        self.descriptor_item_index += 1;

        let cell_ref = cell.as_ref().unwrap();

        drop(Vec::from_raw_parts(cell_ref.addr as *mut u8, cell_ref.length as usize, cell_ref.length as usize));
    }
}

unsafe impl<const S: usize> Send for GuestDriver<S> {}
