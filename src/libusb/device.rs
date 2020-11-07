use crate::libusb::config_descriptor::ConfigDescriptor;
use crate::libusb::device_descriptor::DeviceDescriptor;
use crate::libusb::device_handle::DeviceHandle;
use crate::libusb::error::Error;

#[derive(Debug)]
pub struct Device(core::ptr::NonNull<libusb1_sys::libusb_device>);
impl Device {
    /// # Safety
    /// Assumes the pointer is valid and pointers to a `libusb_device`
    pub const unsafe fn from_libusb(ptr: core::ptr::NonNull<libusb1_sys::libusb_device>) -> Device {
        Device(ptr)
    }

    pub fn active_config_descriptor(&self) -> Result<ConfigDescriptor, Error> {
        let mut out: *const libusb1_sys::libusb_config_descriptor = core::ptr::null_mut();
        try_unsafe!(libusb1_sys::libusb_get_active_config_descriptor(
            self.0.as_ptr(),
            &mut out as *mut _
        ));
        Ok(unsafe {
            ConfigDescriptor::from_libusb(core::ptr::NonNull::new_unchecked(out as *mut _))
        })
    }
    pub fn device_address(&self) -> u8 {
        unsafe { libusb1_sys::libusb_get_device_address(self.0.as_ptr()) }
    }

    pub fn device_descriptor(&self) -> Result<DeviceDescriptor, Error> {
        let mut out: core::mem::MaybeUninit<libusb1_sys::libusb_device_descriptor> =
            core::mem::MaybeUninit::uninit();
        try_unsafe!(libusb1_sys::libusb_get_device_descriptor(
            self.0.as_ptr() as *const _,
            out.as_mut_ptr()
        ));
        Ok(unsafe { DeviceDescriptor::from(out.assume_init()) })
    }
    pub fn open(&self) -> Result<DeviceHandle, Error> {
        let mut out = core::ptr::null_mut();
        try_unsafe!(libusb1_sys::libusb_open(self.0.as_ptr(), &mut out));
        debug_assert!(!out.is_null(), "null libusb device handle ptr");
        Ok(unsafe { DeviceHandle::from_libusb(core::ptr::NonNull::new_unchecked(out)) })
    }
    /// Leak the `Device` without calling `libusb_unref_device`.
    pub fn leak(self) {
        core::mem::forget(self)
    }
    pub fn libusb_ptr(&self) -> core::ptr::NonNull<libusb1_sys::libusb_device> {
        self.0
    }
}
impl Drop for Device {
    fn drop(&mut self) {
        unsafe { libusb1_sys::libusb_unref_device(self.0.as_ptr()) }
    }
}

#[derive(Debug)]
pub struct DeviceList {
    ptr: core::ptr::NonNull<*mut libusb1_sys::libusb_device>,
    len: usize,
}
impl DeviceList {
    /// # Safety
    /// Assumes the pointer is valid and pointers to a list of devices
    pub const unsafe fn from_libusb(
        ptr: core::ptr::NonNull<*mut libusb1_sys::libusb_device>,
        len: usize,
    ) -> DeviceList {
        DeviceList { ptr, len }
    }
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
    pub fn len(&self) -> usize {
        self.len
    }
    pub fn get(&self, pos: usize) -> Option<Device> {
        if pos < self.len {
            Some(unsafe {
                let ptr = *self.ptr.as_ptr().add(pos);
                debug_assert!(!ptr.is_null(), "null device ptr");
                libusb1_sys::libusb_ref_device(ptr);
                Device::from_libusb(core::ptr::NonNull::new_unchecked(ptr))
            })
        } else {
            None
        }
    }
    pub fn iter(&self) -> DeviceListIter<'_> {
        DeviceListIter { list: self, pos: 0 }
    }
}
impl Drop for DeviceList {
    fn drop(&mut self) {
        unsafe { libusb1_sys::libusb_free_device_list(self.ptr.as_ptr(), 1) }
    }
}
pub struct DeviceListIter<'a> {
    pub list: &'a DeviceList,
    pos: usize,
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
