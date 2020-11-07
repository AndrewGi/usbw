use crate::libusb::endpoint_descriptor::EndpointDescriptors;

#[derive(Copy, Clone)]
pub struct Interfaces<'a>(pub &'a [libusb1_sys::libusb_interface]);
impl<'a> Interfaces<'a> {
    pub fn from_slice(slice: &'a [libusb1_sys::libusb_interface]) -> Interfaces<'a> {
        Interfaces(slice)
    }
    /// # Safety
    /// Assumes the pointer is valid and pointers to a list of interfaces
    pub unsafe fn from_ptr(ptr: *mut libusb1_sys::libusb_interface, len: usize) -> Interfaces<'a> {
        Interfaces(core::slice::from_raw_parts(ptr, len))
    }

    pub fn iter(&self) -> impl Iterator<Item = Interface<'a>> {
        self.0.iter().map(Interface)
    }
}

pub struct Interface<'a>(pub &'a libusb1_sys::libusb_interface);
impl<'a> Interface<'a> {
    pub fn descriptors(&self) -> InterfaceDescriptors<'_> {
        let ptr = self.0.altsetting;
        let len = self.0.num_altsetting as usize;
        InterfaceDescriptors(unsafe { core::slice::from_raw_parts(ptr, len) })
    }
}

#[derive(Copy, Clone)]
pub struct InterfaceDescriptors<'a>(pub &'a [libusb1_sys::libusb_interface_descriptor]);
impl<'a> InterfaceDescriptors<'a> {
    pub fn iter(&self) -> impl Iterator<Item = InterfaceDescriptor<'_>> {
        self.0.iter().map(InterfaceDescriptor)
    }
}
#[derive(Copy, Clone)]
pub struct InterfaceDescriptor<'a>(pub &'a libusb1_sys::libusb_interface_descriptor);

impl<'a> InterfaceDescriptor<'a> {
    /// Returns the interface's number.
    pub fn interface_number(&self) -> u8 {
        self.0.bInterfaceNumber
    }

    /// Returns the alternate setting number.
    pub fn setting_number(&self) -> u8 {
        self.0.bAlternateSetting
    }

    /// Returns the interface's class code.
    pub fn class_code(&self) -> u8 {
        self.0.bInterfaceClass
    }

    /// Returns the interface's sub class code.
    pub fn sub_class_code(&self) -> u8 {
        self.0.bInterfaceSubClass
    }

    /// Returns the interface's protocol code.
    pub fn protocol_code(&self) -> u8 {
        self.0.bInterfaceProtocol
    }

    /// Returns the index of the string descriptor that describes the interface.
    pub fn description_string_index(&self) -> Option<u8> {
        match self.0.iInterface {
            0 => None,
            n => Some(n),
        }
    }

    /// Returns the number of endpoints belonging to this interface.
    pub fn num_endpoints(&self) -> u8 {
        self.0.bNumEndpoints
    }

    /// Returns an iterator over the interface's endpoint descriptors.
    pub fn endpoint_descriptors(&self) -> EndpointDescriptors<'_> {
        let endpoints =
            unsafe { core::slice::from_raw_parts(self.0.endpoint, self.0.bNumEndpoints as usize) };

        EndpointDescriptors(endpoints)
    }

    /// Returns the unknown 'extra' bytes that libusb does not understand.
    pub fn extra(&self) -> Option<&[u8]> {
        unsafe {
            match (*self.0).extra_length {
                len if len > 0 => Some(core::slice::from_raw_parts((*self.0).extra, len as usize)),
                _ => None,
            }
        }
    }
}
