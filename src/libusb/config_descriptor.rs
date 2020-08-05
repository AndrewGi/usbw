use crate::libusb::interface_descriptor::Interfaces;

pub struct ConfigDescriptor(core::ptr::NonNull<libusb1_sys::libusb_config_descriptor>);
impl ConfigDescriptor {
    pub unsafe fn from_libusb(
        ptr: core::ptr::NonNull<libusb1_sys::libusb_config_descriptor>,
    ) -> ConfigDescriptor {
        ConfigDescriptor(ptr)
    }
    pub fn number(&self) -> u8 {
        self.inner_ref().bConfigurationValue
    }
    /// Returns max power in milliamps
    pub fn max_power(&self) -> u16 {
        u16::from(self.inner_ref().bMaxPower) * 2
    }
    /// Indicates if the device is self-powered in this configuration.
    pub fn self_powered(&self) -> bool {
        self.inner_ref().bmAttributes & 0x40 != 0
    }

    /// Indicates if the device has remote wakeup capability in this configuration.
    pub fn remote_wakeup(&self) -> bool {
        self.inner_ref().bmAttributes & 0x20 != 0
    }

    /// Returns the index of the string descriptor that describes the configuration.
    pub fn description_string_index(&self) -> Option<u8> {
        match self.inner_ref().iConfiguration {
            0 => None,
            n => Some(n),
        }
    }

    /// Returns the number of interfaces for this configuration.
    pub fn num_interfaces(&self) -> u8 {
        self.inner_ref().bNumInterfaces
    }

    /// Returns the unknown 'extra' bytes that libusb does not understand.
    pub fn extra(&self) -> Option<&[u8]> {
        unsafe {
            match self.inner_ref().extra_length {
                len if len > 0 => Some(core::slice::from_raw_parts(
                    self.inner_ref().extra,
                    len as usize,
                )),
                _ => None,
            }
        }
    }
    pub fn interfaces(&self) -> Interfaces<'_> {
        let ptr = self.inner_ref().interface;
        let len = self.inner_ref().bNumInterfaces;
        Interfaces(unsafe { core::slice::from_raw_parts(ptr, len.into()) })
    }
    pub fn inner_ref(&self) -> &libusb1_sys::libusb_config_descriptor {
        unsafe { self.0.as_ref() }
    }
}
impl Drop for ConfigDescriptor {
    fn drop(&mut self) {
        unsafe { libusb1_sys::libusb_free_config_descriptor(self.0.as_ptr()) }
    }
}

impl core::fmt::Debug for ConfigDescriptor {
    fn fmt(&self, fmt: &mut core::fmt::Formatter) -> Result<(), core::fmt::Error> {
        let mut debug = fmt.debug_struct("ConfigDescriptor");

        let descriptor = self.inner_ref();

        debug.field("bLength", &descriptor.bLength);
        debug.field("bDescriptorType", &descriptor.bDescriptorType);
        debug.field("wTotalLength", &descriptor.wTotalLength);
        debug.field("bNumInterfaces", &descriptor.bNumInterfaces);
        debug.field("bConfigurationValue", &descriptor.bConfigurationValue);
        debug.field("iConfiguration", &descriptor.iConfiguration);
        debug.field("bmAttributes", &descriptor.bmAttributes);
        debug.field("bMaxPower", &descriptor.bMaxPower);
        debug.field("extra", &self.extra());

        debug.finish()
    }
}
unsafe impl Sync for ConfigDescriptor {}
unsafe impl Send for ConfigDescriptor {}
