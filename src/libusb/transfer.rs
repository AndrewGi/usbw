use crate::libusb::error::Error;
use core::convert::TryInto;

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Debug, Hash)]
pub enum Status {
    Completed,
    Error,
    TimedOut,
    Cancelled,
    Stall,
    NoDevice,
    Overflow,
}

#[derive(Debug)]
pub struct Transfer(core::ptr::NonNull<libusb1_sys::libusb_transfer>);
impl Transfer {
    pub fn new(iso_packets: usize) -> Transfer {
        Transfer(
            core::ptr::NonNull::new(unsafe {
                libusb1_sys::libusb_alloc_transfer(iso_packets as i32)
            })
            .expect("null libusb transfer ptr"),
        )
    }
    pub fn libusb_inner(&self) -> core::ptr::NonNull<libusb1_sys::libusb_transfer> {
        self.0
    }
    pub fn submit(&self) -> Result<(), Error> {
        try_unsafe!(libusb1_sys::libusb_submit_transfer(self.0.as_ptr()));
        Ok(())
    }
    pub fn cancel(&self) -> Result<(), Error> {
        try_unsafe!(libusb1_sys::libusb_cancel_transfer(self.0.as_ptr()));
        Ok(())
    }
    pub fn set_stream_id(&self, id: u32) {
        try_unsafe!(libusb1_sys::libusb_transfer_set_stream_id(
            self.0.as_ptr(),
            id
        ))
    }
    pub fn get_stream_id(&self) -> u32 {
        unsafe { libusb1_sys::libusb_transfer_get_stream_id(self.0.as_ptr()) }
    }
    pub unsafe fn from_libusb(ptr: core::ptr::NonNull<libusb1_sys::libusb_transfer>) -> Transfer {
        Transfer(ptr)
    }
    pub fn set_timeout(&mut self, timeout: core::time::Duration) {
        unsafe { self.0.as_mut() }.timeout = timeout.as_millis().try_into().unwrap_or(u32::MAX)
    }
}
impl Drop for Transfer {
    fn drop(&mut self) {
        unsafe { libusb1_sys::libusb_free_transfer(self.0.as_ptr()) }
    }
}
