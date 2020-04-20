//! USB HCI Endpoints. Used Internally.
use crate::ConversionError;
use core::convert::TryFrom;

#[repr(u8)]
pub enum EndpointAddress {
    /// HCI Command. Endpoint type Control
    HCICommand = 0x00,
    /// HCI Event. Endpoint type Interrupt
    HCIEvents = 0x81,
    /// ACL Bulk Data In. Endpoint type Bulk In
    ACLBulkIn = 0x82,
    /// ACL Bulk Data Out. Endpoint type Bulk Out
    ACLBulkOut = 0x02,
}
impl From<EndpointAddress> for u8 {
    fn from(address: EndpointAddress) -> Self {
        address as u8
    }
}
impl TryFrom<u8> for EndpointAddress {
    type Error = ConversionError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0x00 => Ok(EndpointAddress::HCICommand),
            0x02 => Ok(EndpointAddress::ACLBulkOut),
            0x81 => Ok(EndpointAddress::HCIEvents),
            0x82 => Ok(EndpointAddress::ACLBulkIn),
            _ => Err(ConversionError(())),
        }
    }
}
