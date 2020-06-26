pub fn main() -> Result<(), usbw::libusb::error::Error> {
    println!("start");
    let context = usbw::libusb::context::Context::new()?;
    context.hotplug_register_callback(
        |d, e| {
            println!("{:?} {:?}", d, e);
            true
        },
        usbw::libusb::hotplug::Event::Both,
        usbw::libusb::hotplug::Flags::Enumerate,
        None,
        None,
        None,
    )?;
    loop {
        context.handle_events()?;
    }
}
