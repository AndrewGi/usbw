#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Debug, Hash)]
#[repr(i32)]
pub enum Event {
    DeviceArrived = 1,
    DeviceLeft = 2,
    Both = 3,
}
pub enum Flags {
    NoFlags = 0,
    Enumerate = 1,
}
pub struct CallbackHandle(libusb1_sys::libusb_hotplug_callback_handle);
impl CallbackHandle {}
