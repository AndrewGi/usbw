use futures_util::StreamExt;
use usbw::libusb::device::Device;
use usbw::libusb::error::Error;

const WIRELESS_CONTROLLER_CLASS: u8 = 0xE0;
const SUBCLASS: u8 = 0x01;
const BLUETOOTH_PROGRAMMING_INTERFACE_PROTOCOL: u8 = 0x01;
pub fn has_bluetooth_interface(device: &Device) -> Result<bool, Error> {
    match device.active_config_descriptor() {
        Ok(config) => Ok(config
            .interfaces()
            .iter()
            .next()
            .and_then(|i| {
                i.descriptors().iter().next().map(|d| {
                    d.class_code() == WIRELESS_CONTROLLER_CLASS
                        && d.sub_class_code() == SUBCLASS
                        && d.protocol_code() == BLUETOOTH_PROGRAMMING_INTERFACE_PROTOCOL
                })
            })
            .unwrap_or(false)),
        Err(usbw::libusb::error::Error::NotFound) => Ok(false),
        Err(e) => Err(e),
    }
}

pub fn bluetooth_adapters<'a>(
    i: impl Iterator<Item = Device> + 'a,
) -> impl Iterator<Item = Result<Device, Error>> + 'a {
    i.filter_map(|d| match has_bluetooth_interface(&d) {
        Ok(true) => Some(Ok(d)),
        Ok(false) => None,
        Err(e) => Some(Err(e)),
    })
}

pub fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut runtime = tokio::runtime::Builder::new()
        .enable_all()
        .build()
        .expect("can't make async runtime");
    runtime.block_on(main_async())?;
    Ok(())
}
pub const HCI_EVENT_ENDPOINT: u8 = 0x81;
async fn main_async() -> Result<(), Box<dyn std::error::Error>> {
    println!("starting");
    let context = usbw::libusb::context::Context::default()?;
    println!("context made");
    let device_list = context.device_list();
    for d in bluetooth_adapters(device_list.iter()) {
        println!("{:?}", d?.device_descriptor()?);
    }
    println!("opening first device...");
    let mut devices = bluetooth_adapters(device_list.iter());
    let adapter = loop {
        let device = devices
            .next()
            .ok_or_else(|| String::from("Device Not Found"))??;
        println!("using {:?}", device);
        match device.open() {
            Ok(adapter) => break adapter,
            Err(usbw::libusb::error::Error::NotSupported) => (),
            Err(e) => Err(e)?,
        }
    };

    let mut adapter = adapter;
    println!("reset");
    adapter.reset()?;
    println!("active configuration {:?}", adapter.active_configuration());
    println!("claim");
    adapter.claim_interface(0)?;
    adapter.control_write(
        0x20,
        0,
        0,
        0,
        &[0x03, 0x0C, 0x00],
        core::time::Duration::from_secs(1),
    )?;
    println!("reading a byte");
    let mut out = [0; 7];
    adapter.interrupt_read(
        HCI_EVENT_ENDPOINT,
        &mut out,
        core::time::Duration::from_secs(1),
    )?;
    println!("out {:?}", out);
    Ok(())
}
