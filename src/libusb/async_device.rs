use crate::libusb::device_handle::DeviceHandle;
use crate::libusb::error::Error;
use crate::libusb::transfer::{ControlSetup, SafeTransfer, Transfer};
use driver_async::asyncs::sync::oneshot;
use std::convert::TryInto;
/// The Synchronous libusb interface converted to rust async. Warning, each function will
/// allocate a `Transfer` and a buffer for any data + `ControlSetup::SIZE`.
pub struct AsyncDevice {
    handle: DeviceHandle,
}
struct CallbackData {
    completed: Option<oneshot::Sender<()>>,
}
impl CallbackData {
    pub fn new(tx: oneshot::Sender<()>) -> Self {
        Self {
            completed: Some(tx),
        }
    }
    pub fn send_completed(&mut self) {
        self.completed.take().map(|c| c.send(()).ok());
    }
}
impl Drop for CallbackData {
    fn drop(&mut self) {
        self.send_completed()
    }
}
impl AsyncDevice {
    pub fn from_device(handle: DeviceHandle) -> AsyncDevice {
        AsyncDevice { handle }
    }
    extern "system" fn system_callback(transfer: *mut libusb1_sys::libusb_transfer) {
        let mut transfer = unsafe {
            Transfer::from_libusb(
                core::ptr::NonNull::new(transfer).expect("null transfer ptr in callback"),
            )
        };
        Self::callback(&mut transfer);
        core::mem::forget(transfer)
    }
    fn callback(transfer: &mut Transfer) {
        if transfer.libusb_ref().user_data.is_null() {
            return;
        }
        let callback_data = unsafe { transfer.cast_userdata_mut::<CallbackData>() };
        callback_data.send_completed();
    }
    pub async fn control_read(
        &self,
        request_type: u8,
        request: u8,
        value: u16,
        index: u16,
        data: &mut [u8],
        timeout: core::time::Duration,
    ) -> Result<usize, Error> {
        if request_type & libusb1_sys::constants::LIBUSB_ENDPOINT_DIR_MASK
            != libusb1_sys::constants::LIBUSB_ENDPOINT_IN
        {
            return Err(Error::InvalidParam);
        }
        // Allocate Transfer
        let mut transfer = Transfer::new(0);
        // Allocate buffer for data (have to allocate data.len() + ControlSetup::SIZE sadly)
        let mut buf = Vec::with_capacity(data.len() + ControlSetup::SIZE).into_boxed_slice();
        // Allocate CallbackData that enables Async
        let (tx, completed_wait) = oneshot::channel();
        let mut callback = Box::new(CallbackData::new(tx));
        // Fill transfer with control parameters
        transfer.set_user_data(&mut callback as &mut CallbackData as *mut CallbackData);
        let mut transfer = SafeTransfer::new(transfer, buf.as_mut());
        transfer.set_control_setup(
            &self.handle,
            ControlSetup {
                request_type,
                request,
                value,
                index,
                len: data.len().try_into().expect("too much data"),
            },
        );
        // Send the transfer off
        unsafe { transfer.transfer_mut().submit() }?;
        // TODO: Check if sender is dropped
        completed_wait
            .await
            .expect("sender was dropped, Andrew need to fix this");
        let len = transfer.transfer_ref().try_actual_length()?;
        Ok(0)
    }
}
