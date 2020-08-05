use usbw::libusb;
pub fn main() -> Result<(), Box<dyn std::error::Error + 'static>> {
    let context = libusb::context::Context::new()?;
    for device in context.device_list().iter() {
        if let Ok(descriptor) = device.device_descriptor() {
            println!(
                "vid: {:04X} pid: {:04X}",
                descriptor.device_identifier().vendor_id.0,
                descriptor.device_identifier().product_id.0
            )
        }
    }
    Ok(())
}
