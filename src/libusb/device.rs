use crate::libusb::error::Error;

#[derive(Debug)]
pub struct Device(core::ptr::NonNull<libusb1_sys::libusb_device>);
impl Device {
    pub const unsafe fn from_libusb(ptr: core::ptr::NonNull<libusb1_sys::libusb_device>) -> Device {
        Device(ptr)
    }
    pub fn open(&self) -> Result<DeviceHandle, Error> {
        let mut out = core::ptr::null_mut();
        try_unsafe!(libusb1_sys::libusb_open(self.0.as_ptr(), &mut out));
        Ok(DeviceHandle(
            core::ptr::NonNull::new(out).expect("null libusb device handle ptr"),
        ))
    }
    /// Leak the `Device` without calling `libusb_unref_device`.
    pub fn leak(self) {
        core::mem::forget(self)
    }
    pub fn libusb_ptr(&self) -> core::ptr::NonNull<libusb1_sys::libusb_device> {
        self.0
    }
}
impl Clone for Device {
    fn clone(&self) -> Self {
        unsafe { libusb1_sys::libusb_ref_device(self.0.as_ptr()) };
        Device(self.0)
    }
}
impl Drop for Device {
    fn drop(&mut self) {
        unsafe { libusb1_sys::libusb_unref_device(self.0.as_ptr()) }
    }
}
pub struct DeviceHandle(core::ptr::NonNull<libusb1_sys::libusb_device_handle>);
impl DeviceHandle {
    pub fn close(self) {
        drop(self)
    }
}
impl Drop for DeviceHandle {
    fn drop(&mut self) {
        unsafe { libusb1_sys::libusb_close(self.0.as_ptr()) }
    }
}
#[derive(Debug)]
pub struct DeviceList {
    ptr: core::ptr::NonNull<*mut libusb1_sys::libusb_device>,
    len: usize,
}
impl DeviceList {
    pub const unsafe fn from_libusb(
        ptr: core::ptr::NonNull<*mut libusb1_sys::libusb_device>,
        len: usize,
    ) -> DeviceList {
        DeviceList { ptr, len }
    }
    pub fn len(&self) -> usize {
        self.len
    }
    pub fn get(&self, pos: usize) -> Option<Device> {
        if pos < self.len {
            Some(unsafe {
                let ptr = *self.ptr.as_ptr().add(pos);
                debug_assert!(!ptr.is_null(), "null device ptr");
                Device::from_libusb(core::ptr::NonNull::new_unchecked(ptr))
            })
        } else {
            None
        }
    }
    pub fn iter(&self) -> DeviceListIter {
        DeviceListIter::new(self)
    }
}
impl Drop for DeviceList {
    fn drop(&mut self) {
        unsafe { libusb1_sys::libusb_free_device_list(self.ptr.as_ptr(), 1) }
    }
}
#[derive(Copy, Clone, Debug)]
pub struct DeviceListIter<'a> {
    list: &'a DeviceList,
    pos: usize,
}
impl<'a> DeviceListIter<'a> {
    pub fn new(list: &'a DeviceList) -> DeviceListIter {
        DeviceListIter { list, pos: 0 }
    }
    pub fn list(&self) -> &'a DeviceList {
        self.list
    }
    pub fn remaining(&self) -> usize {
        self.list.len - self.pos
    }
}
impl<'a> core::iter::Iterator for DeviceListIter<'a> {
    type Item = Device;

    fn next(&mut self) -> Option<Self::Item> {
        if self.pos >= self.list.len() {
            None
        } else {
            let out = self.list.get(self.pos)?;
            self.pos += 1;
            Some(out)
        }
    }
}
