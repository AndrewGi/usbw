use crate::libusb::device::DeviceList;
use crate::libusb::error::Error;

#[derive(Copy, Clone, Debug)]
#[repr(i32)]
pub enum LogLevel {
    None = 0,
    Error = 1,
    Warning = 2,
    Info = 3,
    Debug = 4,
}

#[derive(Debug)]
pub struct Context(core::ptr::NonNull<libusb1_sys::libusb_context>);
impl Context {
    pub fn new() -> Result<Context, Error> {
        let mut context = core::ptr::null_mut();
        try_unsafe!(libusb1_sys::libusb_init(&mut context));
        Ok(Context(
            core::ptr::NonNull::new(context).expect("libusb null context ptr"),
        ))
    }
    pub fn set_debug(&self, level: LogLevel) {
        unsafe { libusb1_sys::libusb_set_debug(self.0.as_ptr(), level as i32) }
    }
    pub fn device_list(&self) -> DeviceList {
        let mut out = core::ptr::null();
        let len = unsafe { libusb1_sys::libusb_get_device_list(self.0.as_ptr(), &mut out) };
        unsafe {
            DeviceList::from_libusb(
                core::ptr::NonNull::new_unchecked(out as *mut *mut libusb1_sys::libusb_device),
                len as usize,
            )
        }
    }
}
impl Drop for Context {
    fn drop(&mut self) {
        unsafe { libusb1_sys::libusb_exit(self.0.as_ptr()) }
    }
}
