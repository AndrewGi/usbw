use crate::version::Version;


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
