use std::mem::ManuallyDrop;

use crate::{epoll::Epoll, io_uring::{IOUring, create_rings}};

use self::{virtqueue::VirtQueue, guest_driver::GuestDriver, device_driver::DeviceDriver};
use libc::{pipe2, O_NONBLOCK};

pub mod device_register;
pub mod virtqueue;
pub mod guest_driver;
pub mod device_driver;


pub fn create_epoll_queue<const S: usize>() -> (GuestDriver<S, Epoll>, DeviceDriver<S, Epoll>) {
    let mut core_virt_queue = ManuallyDrop::new(Box::new(VirtQueue::<S>::new_with_size()));

    let mut guest_to_device = [-1; 2];
    let mut device_to_guest = [-1; 2];

    unsafe {
        pipe2(guest_to_device.as_mut_ptr(), O_NONBLOCK);
        pipe2(device_to_guest.as_mut_ptr(), O_NONBLOCK);
    }

    (
        GuestDriver::new_epoll(core_virt_queue.as_mut(), device_to_guest[0], guest_to_device[1]),
        DeviceDriver::new_epoll(core_virt_queue.as_mut(), guest_to_device[0], device_to_guest[1])
    )
}

pub fn create_io_uring_queue<const S: usize>() -> (GuestDriver<S, IOUring>, DeviceDriver<S, IOUring>) {
    let mut core_virt_queue = ManuallyDrop::new(Box::new(VirtQueue::<S>::new_with_size()));

    let (guest_poller, device_poller) = create_rings(12);

    (
        GuestDriver::new(core_virt_queue.as_mut(), guest_poller),
        DeviceDriver::new(core_virt_queue.as_mut(), device_poller)
    )
}
