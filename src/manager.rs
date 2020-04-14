use crate::device::DeviceList;
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
    pub fn devices<'a>(&self) -> Result<DeviceList, Error> {
        Ok(self.context.devices()?.into())
    }
}
