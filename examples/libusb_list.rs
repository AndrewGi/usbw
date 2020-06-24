pub fn main() -> Result<(), usbw::libusb::error::Error> {
    println!("start");
    let context = usbw::libusb::context::Context::new()?;
    let list = context.device_list();
    let iter = list.iter();
    for device in iter {
        println!("device {:?}", &device)
    }
    println!("done");
    Ok(())
}
