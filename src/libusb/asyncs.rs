#![allow(unused_variables)]
use crate::libusb::device::Device;

pub struct AsyncDevice {
    device: Device,
}
impl AsyncDevice {
    pub fn from_device(device: Device) -> AsyncDevice {
        AsyncDevice { device }
    }
}
