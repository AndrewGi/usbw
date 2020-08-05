use crate::device::{DeviceIdentifier, ProductID, VendorID};

pub struct DeviceDescriptor(pub libusb1_sys::libusb_device_descriptor);
impl Clone for DeviceDescriptor {
    fn clone(&self) -> Self {
        // Work around for implementing copy
        unsafe { DeviceDescriptor(core::ptr::read(&self.0 as *const _)) }
    }
}
impl DeviceDescriptor {
    pub fn manufacturer_string_index(&self) -> Option<u8> {
        match self.0.iManufacturer {
            0 => None,
            n => Some(n),
        }
    }

    /// Returns the index of the string descriptor that contains the product name.
    pub fn product_string_index(&self) -> Option<u8> {
        match self.0.iProduct {
            0 => None,
            n => Some(n),
        }
    }

    /// Returns the index of the string descriptor that contains the device's serial number.
    pub fn serial_number_string_index(&self) -> Option<u8> {
        match self.0.iSerialNumber {
            0 => None,
            n => Some(n),
        }
    }

    /// Returns the device's class code.
    pub fn class_code(&self) -> u8 {
        self.0.bDeviceClass
    }

    /// Returns the device's sub class code.
    pub fn sub_class_code(&self) -> u8 {
        self.0.bDeviceSubClass
    }

    /// Returns the device's protocol code.
    pub fn protocol_code(&self) -> u8 {
        self.0.bDeviceProtocol
    }

    pub fn vendor_id(&self) -> VendorID {
        VendorID(self.0.idVendor)
    }
    pub fn product_id(&self) -> ProductID {
        ProductID(self.0.idProduct)
    }
    pub fn device_identifier(&self) -> DeviceIdentifier {
        DeviceIdentifier {
            vendor_id: VendorID(self.0.idVendor),
            product_id: ProductID(self.0.idProduct),
        }
    }
}
impl From<libusb1_sys::libusb_device_descriptor> for DeviceDescriptor {
    fn from(d: libusb1_sys::libusb_device_descriptor) -> Self {
        DeviceDescriptor(d)
    }
}

impl core::fmt::Debug for DeviceDescriptor {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("DeviceDescriptor")
            .field("bLength", &self.0.bLength)
            .field("bDescriptorType", &self.0.bDescriptorType)
            .field("bcdUSB", &self.0.bcdUSB)
            .field("bDeviceClass", &self.0.bDeviceClass)
            .field("bDeviceSubClass", &self.0.bDeviceSubClass)
            .field("bDeviceProtocol", &self.0.bDeviceProtocol)
            .field("bMaxPacketSize0", &self.0.bMaxPacketSize0)
            .field("idVendor", &self.0.idVendor)
            .field("idProduct", &self.0.idProduct)
            .field("bcdDevice", &self.0.bcdDevice)
            .field("iManufacturer", &self.0.iManufacturer)
            .field("iProduct", &self.0.iProduct)
            .field("iSerialNumber", &self.0.iSerialNumber)
            .field("bNumConfigurations", &self.0.bNumConfigurations)
            .finish()
    }
}
