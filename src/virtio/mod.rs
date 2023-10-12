use std::mem::ManuallyDrop;

use self::{virtqueue::VirtQueue, guest_driver::GuestDriver, device_driver::DeviceDriver};

pub mod device_register;
pub mod virtqueue;
pub mod guest_driver;
pub mod device_driver;


pub fn create_queue<const S: usize>() -> (GuestDriver<S>, DeviceDriver<S>) {
    let mut core_virt_queue = ManuallyDrop::new(Box::new(VirtQueue::<S>::new_with_size()));

    (GuestDriver::new_driver(core_virt_queue.as_mut()), DeviceDriver::new_driver(core_virt_queue.as_mut()))
}
