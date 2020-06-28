use crate::libusb::device::DeviceHandle;

pub struct DevMem {
    ptr: core::ptr::NonNull<u8>,
    len: usize,
}
impl DevMem {
    pub fn new(device_handle: DeviceHandle, len: usize) -> Option<DevMem> {
        unimplemented!("libusb1_sys is missing dev_mem_alloc and free")
    }
}
