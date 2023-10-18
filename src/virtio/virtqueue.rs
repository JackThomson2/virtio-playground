use std::{mem::{size_of, ManuallyDrop, MaybeUninit}, sync::atomic::{AtomicU16, Ordering::SeqCst}, ptr};

#[repr(C)]
pub struct DescriptorCell {
    pub addr: u64,
    pub length: u32,
    pub flags: u16,
    pub next: u16,
}

impl Default for DescriptorCell {
    fn default() -> Self {
        Self { addr: 0, length: 0, flags: 0, next: 0 }
    }
}

#[repr(C)]
pub struct Available {
    flags: u16,
    idx: u16,
    ring: *mut u16
}

#[repr(C)]
pub struct UsedCell {
    pub id: u32,
    pub len: u32
}

impl Default for UsedCell {
    fn default() -> Self {
        Self { id: 0, len: 0 }
    }
}

#[repr(C)]
pub struct Used {
    pub flags: u16,
    pub idx: u16,
    pub ring: *mut UsedCell
}

pub struct VirtQueue<const S: usize> {
    pub descriptor_cell: *mut DescriptorCell,
    pub available: *mut Available,
    pub used: *mut Used,
    pub size: u16,
}

type MemoryRange      = ManuallyDrop<Box<[UsedCell]>>;
type MemoryDescriptor = ManuallyDrop<Box<[DescriptorCell]>>;
type MemoryAvailable  = ManuallyDrop<Box<[u16]>>;

impl Available {
    pub unsafe fn get_ring_from_idx(&mut self, idx: u16) -> *mut u16 {
        self.ring.add(idx as usize)
    }

    pub unsafe fn get_idx(&mut self) -> u16 {
        (self.idx as *mut u16).read_volatile()
    }

    pub unsafe fn increment_idx(&mut self, max_size: u16) {
        let new_idx = (self.get_idx() + 1) & max_size - 1;

        (self.idx as *mut u16).write_volatile(new_idx);
    }
}

impl Used {
    pub unsafe fn get_ring_from_idx(&mut self, idx: u16) -> *mut UsedCell {
        self.ring.add(idx as usize)
    }
}

impl<const S: usize> VirtQueue<S> {
    pub fn new_with_size() -> Self {
        let mut used_list: MemoryRange = ManuallyDrop::new(
            Vec::from_iter(
                (0..S).map(|_| Default::default())
            ).into_boxed_slice()
        );
        let mut available_list: MemoryAvailable = ManuallyDrop::new(Vec::from_iter(0..S as u16).into_boxed_slice());
        let mut descriptor_table: MemoryDescriptor = ManuallyDrop::new(
            Vec::from_iter(
                (0..S).map(|_| Default::default())
            ).into_boxed_slice()
        );

        let mut used = ManuallyDrop::new(Box::new(Used {
            flags: 0,
            idx: 0,
            ring: used_list.as_mut_ptr()
        }));

        let mut available = ManuallyDrop::new(Box::new(Available {
            flags: 0,
            idx: 0,
            ring: available_list.as_mut_ptr()
        }));

        Self {
            descriptor_cell: descriptor_table.as_mut_ptr(),
            available: available.as_mut(),
            used: used.as_mut(),
            size: S as u16,
        }
    }

    pub unsafe fn get_descriptor_from_idx(&self, idx: u16) -> &mut DescriptorCell {
        self.descriptor_cell.add(idx as usize).as_mut().unwrap()
    }
}

#[test]
pub fn test_sizes() {
    println!("Size of des cell: {}", size_of::<DescriptorCell>());
    println!("Size of available: {}", size_of::<Available>());
    println!("Size of used cell: {}", size_of::<UsedCell>());
    println!("Size of used: {}", size_of::<Used>());
}
