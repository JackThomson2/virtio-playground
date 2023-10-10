use std::{mem::{size_of, ManuallyDrop, MaybeUninit}, sync::atomic::{AtomicU16, Ordering::SeqCst}, ptr};

#[repr(C)]
pub struct DescriptorCell {
    addr: u64,
    length: u32,
    flags: u16,
    next: u16,
}

impl Default for DescriptorCell {
    fn default() -> Self {
        Self { addr: 0, length: 0, flags: 0, next: 0 }
    }
}

#[repr(C)]
pub struct Available {
    flags: u16,
    idx: AtomicU16,
    ring: *mut u16
}

#[repr(C)]
pub struct UsedCell {
    pub id: u32,
    pub len: u32
}

#[repr(C)]
pub struct Used {
    pub flags: u16,
    pub idx: AtomicU16,
    pub ring: *mut UsedCell
}

pub struct VirtQueue<const S: usize> {
    pub descriptor_cell: *mut DescriptorCell,
    pub available: *mut Available,
    pub used: *mut Used,
    pub size: u16,
}

type MemoryRange      = ManuallyDrop<Box<[MaybeUninit<UsedCell>]>>;
type MemoryDescriptor = ManuallyDrop<Box<[DescriptorCell]>>;
type MemoryAvailable  = ManuallyDrop<Box<[u16]>>;

impl Available {

    pub unsafe fn get_ring_from_idx(&mut self, idx: u16) -> *mut u16 {
        self.ring.add(idx as usize)

    }

    pub fn increment_idx(&mut self, max_size: u16) {
        let current = self.idx.load(SeqCst);

        if current - 1 >= max_size {
            self.idx.store(0, SeqCst);
        }

        self.idx.fetch_add(1, SeqCst);
    }
}

impl Used {

    pub unsafe fn get_ring_from_idx(&mut self, idx: u16) -> *mut UsedCell {
        self.ring.add(idx as usize)
    }

}

impl<const S: usize> VirtQueue<S> {
    pub fn new_with_size() -> Self {
        unsafe {
            let mut used_list: MemoryRange = ManuallyDrop::new(Box::new_uninit_slice(S));
            let mut available_list: MemoryAvailable = ManuallyDrop::new(Vec::from_iter(0..S as u16).into_boxed_slice());
            let mut descriptor_table: MemoryDescriptor = ManuallyDrop::new(
                Vec::from_iter(
                    (0..S).map(|_| Default::default())
                ).into_boxed_slice()
            );

            let mut used = ManuallyDrop::new(Box::new(Used {
                flags: 0,
                idx: AtomicU16::new(0),
                ring: used_list[0].assume_init_mut()
            }));

            let mut available = ManuallyDrop::new(Box::new(Available {
                flags: 0,
                idx: AtomicU16::new(0),
                ring: available_list.as_mut_ptr()
            }));

            Self {
                descriptor_cell: descriptor_table.as_mut_ptr(),
                available: available.as_mut(),
                used: used.as_mut(),
                size: S as u16,
            }
        }
    }

    pub unsafe fn get_descriptor_from_idx(&self, idx: u16) -> &mut DescriptorCell {
        self.descriptor_cell.add(idx as usize).as_mut().unwrap()
    }

    // pub unsafe fn get_descriptor_cell(&mut self) -> Option<*mut DescriptorCell> {
    //     if self.available_count == 0 {
    //         return None;
    //     }

    //     let available_cell = self.available.as_mut().unwrap();

    //     let next_slot = match available_cell.get_next_slot(self.size) {
    //         Some(slot) => slot,
    //         None => return None
    //     };

    //     self.available_count = self.available_count.saturating_sub(1);

    //     let cell_to_give = self.descriptor_cell.add(next_slot as usize);

    //     Some(cell_to_give)
    // }
}

#[test]
pub fn test_sizes() {
    println!("Size of des cell: {}", size_of::<DescriptorCell>());
    println!("Size of available: {}", size_of::<Available>());
    println!("Size of used cell: {}", size_of::<UsedCell>());
    println!("Size of used: {}", size_of::<Used>());
}
