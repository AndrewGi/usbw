use core::fmt;
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum Error {
    /// Input/output error.
    Io,

    /// Invalid parameter.
    InvalidParam,

    /// Access denied (insufficient permissions).
    Access,

    /// No such device (it may have been disconnected).
    NoDevice,

    /// Entity not found.
    NotFound,

    /// Resource busy.
    Busy,

    /// Operation timed out.
    Timeout,

    /// Overflow.
    Overflow,

    /// Pipe error.
    Pipe,

    /// System call interrupted (perhaps due to signal).
    Interrupted,

    /// Insufficient memory.
    NoMem,

    /// Operation not supported or unimplemented on this platform.
    NotSupported,

    /// The device returned a malformed descriptor.
    BadDescriptor,

    /// Other error.
    Other,
}
impl Error {
    pub fn libusb_name(self) -> &'static str {
        unsafe {
            let ptr = libusb1_sys::libusb_error_name(self as i32);
            std::ffi::CStr::from_ptr(ptr)
                .to_str()
                .expect("libusb error name utf-8 error")
        }
    }
    pub fn libusb_description(self) -> &'static str {
        unsafe {
            let ptr = libusb1_sys::libusb_strerror(self as i32);
            std::ffi::CStr::from_ptr(ptr)
                .to_str()
                .expect("libusb error name utf-8 error")
        }
    }
    pub fn as_str(self) -> &'static str {
        match self {
            Error::Io => "Input/Output Error",
            Error::InvalidParam => "Invalid parameter",
            Error::Access => "Access denied (insufficient permissions)",
            Error::NoDevice => "No such device (it may have been disconnected)",
            Error::NotFound => "Entity not found",
            Error::Busy => "Resource busy",
            Error::Timeout => "Operation timed out",
            Error::Overflow => "Overflow",
            Error::Pipe => "Pipe error",
            Error::Interrupted => "System call interrupted (perhaps due to signal)",
            Error::NoMem => "Insufficient memory",
            Error::NotSupported => "Operation not supported or unimplemented on this platform",
            Error::BadDescriptor => "Malformed descriptor",
            Error::Other => "Other error",
        }
    }
}
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::error::Error for Error {}

pub fn from_libusb(err: i32) -> Error {
    match err {
        libusb1_sys::constants::LIBUSB_ERROR_IO => Error::Io,
        libusb1_sys::constants::LIBUSB_ERROR_INVALID_PARAM => Error::InvalidParam,
        libusb1_sys::constants::LIBUSB_ERROR_ACCESS => Error::Access,
        libusb1_sys::constants::LIBUSB_ERROR_NO_DEVICE => Error::NoDevice,
        libusb1_sys::constants::LIBUSB_ERROR_NOT_FOUND => Error::NotFound,
        libusb1_sys::constants::LIBUSB_ERROR_BUSY => Error::Busy,
        libusb1_sys::constants::LIBUSB_ERROR_TIMEOUT => Error::Timeout,
        libusb1_sys::constants::LIBUSB_ERROR_OVERFLOW => Error::Overflow,
        libusb1_sys::constants::LIBUSB_ERROR_PIPE => Error::Pipe,
        libusb1_sys::constants::LIBUSB_ERROR_INTERRUPTED => Error::Interrupted,
        libusb1_sys::constants::LIBUSB_ERROR_NO_MEM => Error::NoMem,
        libusb1_sys::constants::LIBUSB_ERROR_NOT_SUPPORTED => Error::NotSupported,
        libusb1_sys::constants::LIBUSB_ERROR_OTHER => Error::Other,
        _ => Error::Other,
    }
}
macro_rules! try_unsafe {
    ($x:expr) => {
        match unsafe { $x } {
            0 => (),
            err => return Err($crate::libusb::error::from_libusb(err)),
        }
    };
}
