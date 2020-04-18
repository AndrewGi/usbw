use crate::device::{DeviceHandle, DeviceIdentifier, DeviceList};
use crate::error::Error;
use rusb::UsbContext;

pub struct Manager {
    context: rusb::Context,
}
impl Manager {
    pub fn new() -> Result<Self, Error> {
        Ok(Self {
            context: rusb::Context::new()?,
        })
    }
    pub fn devices(&self) -> Result<DeviceList, Error> {
        Ok(self.context.devices()?.into())
    }
    /// Option a USB device by Product ID and Vendor ID.
    pub fn open_device(&self, id: DeviceIdentifier) -> Option<DeviceHandle> {
        self.context
            .open_device_with_vid_pid(id.vid, id.pid)
            .map(DeviceHandle::from)
    }
}
