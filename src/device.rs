use crate::error::Error;
use crate::version::Version;
use rusb::Context;

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Debug, Hash)]
pub struct DeviceIdentifier {
    pub vid: u16,
    pub pid: u16,
}
pub struct StringIndices {
    pub manufacturer: Option<u8>,
    pub product: Option<u8>,
    pub serial_number: Option<u8>,
}
pub struct Codes {
    pub class: u8,
    pub sub_class: u8,
    pub protocol: u8,
}
pub struct Descriptor {
    pub usb_version: Version,
    pub codes: Codes,
    pub max_packet_size: u8,
    pub device_identifier: DeviceIdentifier,
    pub device_version: Version,
    pub string_indices: StringIndices,
    pub num_configurations: u8,
}
impl From<rusb::DeviceDescriptor> for Descriptor {
    fn from(d: rusb::DeviceDescriptor) -> Self {
        Descriptor {
            usb_version: d.usb_version().into(),
            codes: Codes {
                class: d.class_code(),
                sub_class: d.sub_class_code(),
                protocol: d.protocol_code(),
            },
            max_packet_size: d.max_packet_size(),
            device_identifier: DeviceIdentifier {
                vid: d.vendor_id(),
                pid: d.product_id(),
            },
            device_version: d.device_version().into(),
            string_indices: StringIndices {
                manufacturer: d.manufacturer_string_index(),
                product: d.product_string_index(),
                serial_number: d.serial_number_string_index(),
            },
            num_configurations: d.num_configurations(),
        }
    }
}
#[derive(Debug)]
pub struct Device(rusb::Device<rusb::Context>);
impl From<rusb::Device<rusb::Context>> for Device {
    fn from(d: rusb::Device<rusb::Context>) -> Self {
        Self(d)
    }
}
impl Device {
    pub fn device_descriptor(&self) -> Result<Descriptor, Error> {
        let descriptor = self.0.device_descriptor()?;
        Ok(descriptor.into())
    }
}
impl From<rusb::DeviceList<rusb::Context>> for DeviceList {
    fn from(list: rusb::DeviceList<rusb::Context>) -> Self {
        DeviceList(list)
    }
}
pub struct DeviceList(rusb::DeviceList<rusb::Context>);
impl DeviceList {
    pub fn iter(&self) -> PossibleDevices<'_> {
        self.0.iter().into()
    }
}

pub struct PossibleDevices<'a>(rusb::Devices<'a, rusb::Context>);
impl<'a> From<rusb::Devices<'a, rusb::Context>> for PossibleDevices<'a> {
    fn from(devices: rusb::Devices<'a, rusb::Context>) -> Self {
        PossibleDevices(devices)
    }
}
impl<'a> Iterator for PossibleDevices<'a> {
    type Item = Device;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next().map(|d| d.into())
    }
}

pub struct DeviceHandle(rusb::DeviceHandle<rusb::Context>);

impl From<rusb::DeviceHandle<rusb::Context>> for DeviceHandle {
    fn from(handle: rusb::DeviceHandle<Context>) -> Self {
        DeviceHandle(handle)
    }
}
