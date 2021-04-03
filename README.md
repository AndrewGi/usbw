# usbw
`usbw` is a Rust wrapper for usbw. 
The biggest different between `usbw` and the `rusb` crate is `usbw` has support for Asynchronous transfer.
Almost all of the libusb functions and objects are exposed in a safer manner.

# Example
This is an example of using `usbw` for a Bluetooth HCI Adapter. Look at the `examples` folder for more examples.
```rust
use usbw::libusb::device::Device;
use usbw::libusb::error::Error;

const WIRELESS_CONTROLLER_CLASS: u8 = 0xE0;
const SUBCLASS: u8 = 0x01;
const BLUETOOTH_PROGRAMMING_INTERFACE_PROTOCOL: u8 = 0x01;
const HCI_EVENT_ENDPOINT: u8 = 0x81;

async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let context = usbw::libusb::context::Context::default()?;
    let device_list = context.device_list();
    for d in bluetooth_adapters(device_list.iter()) {
    }
    // Get all the devices that have the Bluetooth Adapter endpoints. (not in `usbw`) 
    let mut devices = bluetooth_adapters(device_list.iter());
    let handle = loop {
        let device = devices
            .next()
            .ok_or_else(|| String::from("Device Not Found"))??;
        match device.open() {
            Ok(adapter) => break adapter,
            Err(usbw::libusb::error::Error::NotSupported) => (),
            Err(e) => Err(e)?,
        }
    };
    // `.start_async()` starts a background thread to handle all the `libusb` operations.
    let context = context.start_async();
    let handle = context.make_async_device(handle);
    
    // Reset and claim the Bluetooth Adapter interface.
    let mut adapter = handle;
    adapter.handle_ref().reset()?;
    adapter.handle_mut().claim_interface(0)?;
    
    // Reset the HCI adapter
    adapter
        .control_write(
            0x20,
            0,
            0,
            0,
            &[0x03, 0x0C, 0x00],
            core::time::Duration::from_secs(1),
        )
        .await?;
    // Read the response
    let mut out = [0; 7];
    adapter
        .interrupt_read(
            HCI_EVENT_ENDPOINT,
            &mut out,
            core::time::Duration::from_secs(1),
        )
        .await?;
    Ok(())
}

```