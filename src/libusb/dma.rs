#![allow(unused_variables, dead_code)]
use crate::libusb::device_handle::DeviceHandle;

pub struct DevMem {
    ptr: core::ptr::NonNull<u8>,
    len: usize,
}
impl DevMem {
    pub fn new(_device_handle: DeviceHandle, _len: usize) -> Option<DevMem> {
        unimplemented!("libusb1_sys is missing dev_mem_alloc and free")
    }
}
