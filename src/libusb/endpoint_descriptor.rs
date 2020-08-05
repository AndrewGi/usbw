#[derive(Copy, Clone)]
pub struct EndpointDescriptors<'a>(pub &'a [libusb1_sys::libusb_endpoint_descriptor]);

impl<'a> EndpointDescriptors<'a> {}
