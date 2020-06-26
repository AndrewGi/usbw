use crate::device::{ProductID, VendorID};
use crate::libusb::device::{Device, DeviceList};
use crate::libusb::error::Error;
use crate::libusb::hotplug;
use core::sync::atomic::{AtomicUsize, Ordering};

#[derive(Copy, Clone, Debug)]
#[repr(i32)]
pub enum LogLevel {
    None = 0,
    Error = 1,
    Warning = 2,
    Info = 3,
    Debug = 4,
}
static DEFAULT_CONTEXT_COUNT: AtomicUsize = AtomicUsize::new(0);
#[derive(Debug)]
pub struct Context(*mut libusb1_sys::libusb_context);
impl Context {
    pub fn new() -> Result<Context, Error> {
        let mut context = core::ptr::null_mut();
        try_unsafe!(libusb1_sys::libusb_init(&mut context));
        Ok(Context(context))
    }
    pub fn default() -> Result<Context, Error> {
        // NOOP if default Context already exists
        try_unsafe!(libusb1_sys::libusb_init(core::ptr::null_mut()));
        DEFAULT_CONTEXT_COUNT.fetch_add(1, Ordering::SeqCst);
        Ok(Context(core::ptr::null_mut()))
    }
    pub fn set_debug(&self, level: LogLevel) {
        unsafe { libusb1_sys::libusb_set_debug(self.0, level as i32) }
    }
    pub fn device_list(&self) -> DeviceList {
        let mut out = core::ptr::null();
        let len = unsafe { libusb1_sys::libusb_get_device_list(self.0, &mut out) };
        unsafe {
            DeviceList::from_libusb(
                core::ptr::NonNull::new_unchecked(out as *mut *mut libusb1_sys::libusb_device),
                len as usize,
            )
        }
    }
    pub fn handle_events(&self) -> Result<(), Error> {
        try_unsafe!(libusb1_sys::libusb_handle_events(self.0));
        Ok(())
    }
    pub fn handle_events_timeout(&self, timeout: core::time::Duration) -> Result<(), Error> {
        let time = libc::timeval {
            tv_sec: timeout.as_secs() as i32,
            tv_usec: timeout.subsec_micros() as i32,
        };
        try_unsafe!(libusb1_sys::libusb_handle_events_timeout(self.0, &time));
        Ok(())
    }
    /// Register a hotplug callback. `F` must keep returning `true` for as long as it lives and then
    /// either deregister the callback handle or return `false` from `F`.
    pub fn hotplug_register_callback<F>(
        &self,
        callback: F,
        events: hotplug::Event,
        flag: hotplug::Flags,
        vendor_id: Option<VendorID>,
        product_id: Option<ProductID>,
        device_class: Option<u8>,
    ) -> Result<(), Error>
    where
        F: FnMut(&mut Device, hotplug::Event) -> bool + Send + 'static,
    {
        extern "system" fn call_closure<F>(
            _context: *mut libusb1_sys::libusb_context,
            device: *mut libusb1_sys::libusb_device,
            event: libusb1_sys::libusb_hotplug_event,
            closure: *mut core::ffi::c_void,
        ) -> i32
        where
            F: FnMut(&mut Device, hotplug::Event) -> bool + Send + 'static,
        {
            let event = match event {
                1 => hotplug::Event::DeviceArrived,
                2 => hotplug::Event::DeviceLeft,
                _ => hotplug::Event::Both,
            };
            let closure = closure as *mut F;
            let mut device =
                unsafe { Device::from_libusb(core::ptr::NonNull::new_unchecked(device)) };
            let r = unsafe { &mut *closure }(&mut device, event);
            // We don't wanna libusb_unref_device the device pointer (hotplug callbacks aren't expected to)
            device.leak();
            if r {
                0
            } else {
                // Drop the closure because we're done now
                unsafe { Box::from_raw(closure) };
                1
            }
        }
        const MATCH_ANY: i32 = -1;
        let callback_ptr = Box::into_raw(Box::new(callback)) as *mut core::ffi::c_void;
        try_unsafe!(libusb1_sys::libusb_hotplug_register_callback(
            self.0,
            events as i32,
            flag as i32,
            vendor_id.map(|v| i32::from(v.0)).unwrap_or(MATCH_ANY),
            product_id.map(|p| i32::from(p.0)).unwrap_or(MATCH_ANY),
            device_class.map(i32::from).unwrap_or(MATCH_ANY),
            call_closure::<F>,
            callback_ptr,
            core::ptr::null_mut(),
        ));
        Ok(())
    }
}
unsafe impl Send for Context {}
unsafe impl Sync for Context {}
impl Drop for Context {
    fn drop(&mut self) {
        if self.0.is_null() {
            if DEFAULT_CONTEXT_COUNT.fetch_sub(1, Ordering::SeqCst) != 0 {
                // Not ready to exit default context
                return;
            };
        }
        unsafe { libusb1_sys::libusb_exit(self.0) }
    }
}