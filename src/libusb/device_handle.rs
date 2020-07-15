use crate::libusb::error;
use crate::libusb::error::Error;
use core::convert::TryInto;
#[derive(Debug)]
pub struct DeviceHandle(core::ptr::NonNull<libusb1_sys::libusb_device_handle>);
unsafe impl Send for DeviceHandle {}
unsafe impl Sync for DeviceHandle {}
impl Drop for DeviceHandle {
    fn drop(&mut self) {
        unsafe { libusb1_sys::libusb_close(self.0.as_ptr()) }
    }
}

impl DeviceHandle {
    pub fn set_auto_detach_kernel_driver(&self, enabled: bool) -> Result<(), Error> {
        try_unsafe!(libusb1_sys::libusb_set_auto_detach_kernel_driver(
            self.0.as_ptr(),
            enabled.into()
        ));
        Ok(())
    }
    pub fn control_read(
        &mut self,
        request_type: u8,
        request: u8,
        value: u16,
        index: u16,
        data: &mut [u8],
        timeout: core::time::Duration,
    ) -> Result<usize, Error> {
        if request_type & libusb1_sys::constants::LIBUSB_ENDPOINT_DIR_MASK
            != libusb1_sys::constants::LIBUSB_ENDPOINT_IN
        {
            return Err(Error::InvalidParam);
        }
        let res = unsafe {
            libusb1_sys::libusb_control_transfer(
                self.0.as_ptr(),
                request_type,
                request,
                value,
                index,
                data.as_mut_ptr(),
                data.len()
                    .try_into()
                    .expect("libusb control transfer len overflow"),
                timeout
                    .as_millis()
                    .try_into()
                    .expect("libusb control transfer timeout overflow"),
            )
        };
        if res < 0 {
            Err(error::from_libusb(res))
        } else {
            Ok(res as usize)
        }
    }

    pub fn control_write(
        &mut self,
        request_type: u8,
        request: u8,
        value: u16,
        index: u16,
        data: &[u8],
        timeout: core::time::Duration,
    ) -> Result<usize, Error> {
        if request_type & libusb1_sys::constants::LIBUSB_ENDPOINT_DIR_MASK
            != libusb1_sys::constants::LIBUSB_ENDPOINT_OUT
        {
            return Err(Error::InvalidParam);
        }
        let res = unsafe {
            libusb1_sys::libusb_control_transfer(
                self.0.as_ptr(),
                request_type,
                request,
                value,
                index,
                data.as_ptr() as *mut u8,
                data.len()
                    .try_into()
                    .expect("libusb control transfer len overflow"),
                timeout
                    .as_millis()
                    .try_into()
                    .expect("libusb control transfer timeout overflow"),
            )
        };
        if res < 0 {
            Err(error::from_libusb(res))
        } else {
            Ok(res as usize)
        }
    }

    pub fn bulk_write(
        &mut self,
        endpoint: u8,
        data: &[u8],
        timeout: core::time::Duration,
    ) -> Result<usize, Error> {
        if endpoint & libusb1_sys::constants::LIBUSB_ENDPOINT_DIR_MASK
            != libusb1_sys::constants::LIBUSB_ENDPOINT_OUT
        {
            return Err(Error::InvalidParam);
        }
        let mut transferred = 0;
        unsafe {
            match libusb1_sys::libusb_bulk_transfer(
                self.0.as_ptr(),
                endpoint,
                data.as_ptr() as *mut u8,
                data.len() as i32,
                &mut transferred as *mut i32,
                timeout.as_millis() as u32,
            ) {
                0 => Ok(transferred as usize),
                err if err == libusb1_sys::constants::LIBUSB_ERROR_INTERRUPTED
                    || err == libusb1_sys::constants::LIBUSB_ERROR_TIMEOUT =>
                {
                    if transferred > 0 {
                        Ok(transferred as usize)
                    } else {
                        Err(error::from_libusb(err))
                    }
                }
                err => Err(error::from_libusb(err)),
            }
        }
    }

    pub fn bulk_read(
        &mut self,
        endpoint: u8,
        data: &mut [u8],
        timeout: core::time::Duration,
    ) -> Result<usize, Error> {
        if endpoint & libusb1_sys::constants::LIBUSB_ENDPOINT_DIR_MASK
            != libusb1_sys::constants::LIBUSB_ENDPOINT_IN
        {
            return Err(Error::InvalidParam);
        }
        let mut transferred = 0;
        unsafe {
            match libusb1_sys::libusb_bulk_transfer(
                self.0.as_ptr(),
                endpoint,
                data.as_mut_ptr(),
                data.len() as i32,
                &mut transferred as *mut i32,
                timeout.as_millis() as u32,
            ) {
                0 => Ok(transferred as usize),
                err if err == libusb1_sys::constants::LIBUSB_ERROR_INTERRUPTED
                    || err == libusb1_sys::constants::LIBUSB_ERROR_TIMEOUT =>
                {
                    if transferred > 0 {
                        Ok(transferred as usize)
                    } else {
                        Err(error::from_libusb(err))
                    }
                }
                err => Err(error::from_libusb(err)),
            }
        }
    }
    pub fn interrupt_write(
        &self,
        endpoint: u8,
        data: &[u8],
        timeout: core::time::Duration,
    ) -> Result<usize, Error> {
        if endpoint & libusb1_sys::constants::LIBUSB_ENDPOINT_DIR_MASK
            != libusb1_sys::constants::LIBUSB_ENDPOINT_OUT
        {
            return Err(Error::InvalidParam);
        }
        let mut transferred = 0;
        unsafe {
            match libusb1_sys::libusb_interrupt_transfer(
                self.0.as_ptr(),
                endpoint,
                data.as_ptr() as *mut u8,
                data.len() as i32,
                &mut transferred as *mut i32,
                timeout.as_millis() as u32,
            ) {
                0 => Ok(transferred as usize),
                err if err == libusb1_sys::constants::LIBUSB_ERROR_INTERRUPTED => {
                    if transferred > 0 {
                        Ok(transferred as usize)
                    } else {
                        Err(error::from_libusb(err))
                    }
                }
                err => Err(error::from_libusb(err)),
            }
        }
    }
    pub fn interrupt_read(
        &self,
        endpoint: u8,
        data: &mut [u8],
        timeout: core::time::Duration,
    ) -> Result<usize, Error> {
        if endpoint & libusb1_sys::constants::LIBUSB_ENDPOINT_DIR_MASK
            != libusb1_sys::constants::LIBUSB_ENDPOINT_IN
        {
            return Err(Error::InvalidParam);
        }
        let mut transferred = 0;
        unsafe {
            match libusb1_sys::libusb_interrupt_transfer(
                self.0.as_ptr(),
                endpoint,
                data.as_mut_ptr(),
                data.len() as i32,
                &mut transferred as *mut i32,
                timeout.as_millis() as u32,
            ) {
                0 => Ok(transferred as usize),
                err if err == libusb1_sys::constants::LIBUSB_ERROR_INTERRUPTED => {
                    if transferred > 0 {
                        Ok(transferred as usize)
                    } else {
                        Err(error::from_libusb(err))
                    }
                }
                err => Err(error::from_libusb(err)),
            }
        }
    }
    pub const unsafe fn from_libusb(
        ptr: core::ptr::NonNull<libusb1_sys::libusb_device_handle>,
    ) -> DeviceHandle {
        DeviceHandle(ptr)
    }
    pub fn close(self) {
        drop(self)
    }
    pub fn reset(&self) -> Result<(), Error> {
        try_unsafe!(libusb1_sys::libusb_reset_device(self.0.as_ptr()));
        Ok(())
    }
}
