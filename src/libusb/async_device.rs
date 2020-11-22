use crate::libusb::device::Device;
use crate::libusb::device_handle::DeviceHandle;
use crate::libusb::error::Error;
use crate::libusb::transfer::{
    ControlSetup, SafeTransfer, Transfer, TransferType, TransferWithBuf,
};
use driver_async::asyncs::sync::oneshot;
use std::convert::TryInto;

/// The Synchronous libusb interface converted to rust async. Warning, each function will
/// allocate a `Transfer` and a buffer for any data + `ControlSetup::SIZE`.
pub struct AsyncDevice {
    pub(crate) handle: DeviceHandle,
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
#[derive(Copy, Clone, Debug)]
pub enum BulkType {
    Bulk,
    Interrupt,
}
impl BulkType {
    pub fn transfer_type(self) -> TransferType {
        match self {
            BulkType::Bulk => TransferType::Bulk,
            BulkType::Interrupt => TransferType::Interrupt,
        }
    }
}
impl Drop for CallbackData {
    fn drop(&mut self) {
        self.send_completed()
    }
}
impl AsyncDevice {
    /// # Safety
    /// Will block if a `AsyncContext` is running with the device's context
    pub unsafe fn from_device(handle: DeviceHandle) -> AsyncDevice {
        AsyncDevice { handle }
    }
    extern "system" fn system_callback(transfer: *mut libusb1_sys::libusb_transfer) {
        println!("callback");
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
        transfer.libusb_mut().user_data = core::ptr::null_mut();
    }
    pub fn handle_ref(&self) -> &DeviceHandle {
        &self.handle
    }
    pub fn handle_mut(&mut self) -> &mut DeviceHandle {
        &mut self.handle
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
        let mut transfer = SafeTransfer::from_buf(vec![0_u8; data.len() + ControlSetup::SIZE]);
        transfer.set_timeout(timeout);
        // Fill transfer with control parameters
        transfer.set_control_setup(ControlSetup {
            request_type,
            request,
            value,
            index,
            len: data.len().try_into().expect("too much data"),
        });
        let len = transfer.submit_write(self).await?;
        data[..len].copy_from_slice(&transfer.control_data_ref()[..len]);
        Ok(len)
    }
    pub async fn control_write(
        &self,
        request_type: u8,
        request: u8,
        value: u16,
        index: u16,
        data: &[u8],
        timeout: core::time::Duration,
    ) -> Result<usize, Error> {
        // Allocate Transfer
        let mut transfer = Transfer::new(0);
        // Allocate buffer for data (have to allocate data.len() + ControlSetup::SIZE sadly)
        let mut buf = vec![0; data.len() + ControlSetup::SIZE].into_boxed_slice();
        self.control_write_transfer(
            TransferWithBuf::new(&mut transfer, buf.as_mut()),
            request_type,
            request,
            value,
            index,
            data,
            timeout,
        )
        .await
    }
    pub async fn bulk_type_write(
        &self,
        bulk_type: BulkType,
        endpoint: u8,
        data: &[u8],
        timeout: core::time::Duration,
    ) -> Result<usize, Error> {
        let mut transfer = SafeTransfer::from_buf(data);
        transfer.set_type(bulk_type.into());
        transfer.set_endpoint(endpoint);
        transfer.set_timeout(timeout);
        transfer.submit_write(self).await
    }

    pub async fn bulk_type_read(
        &self,
        bulk_type: BulkType,
        endpoint: u8,
        data: &mut [u8],
        timeout: core::time::Duration,
    ) -> Result<usize, Error> {
        let mut transfer = SafeTransfer::from_buf(data);
        transfer.set_type(bulk_type.into());
        transfer.set_endpoint(endpoint);
        transfer.set_timeout(timeout);
        transfer.submit_read(self).await
    }
    pub async fn bulk_write(
        &self,
        endpoint: u8,
        data: &[u8],
        timeout: core::time::Duration,
    ) -> Result<usize, Error> {
        self.bulk_type_write(BulkType::Bulk, endpoint, data, timeout)
            .await
    }
    pub async fn interrupt_write(
        &self,
        endpoint: u8,
        data: &[u8],
        timeout: core::time::Duration,
    ) -> Result<usize, Error> {
        self.bulk_type_write(BulkType::Interrupt, endpoint, data, timeout)
            .await
    }
    pub async fn bulk_read(
        &mut self,
        endpoint: u8,
        data: &mut [u8],
        timeout: core::time::Duration,
    ) -> Result<usize, Error> {
        self.bulk_type_read(BulkType::Bulk, endpoint, data, timeout)
            .await
    }
    pub async fn interrupt_read(
        &self,
        endpoint: u8,
        data: &mut [u8],
        timeout: core::time::Duration,
    ) -> Result<usize, Error> {
        self.bulk_type_read(BulkType::Interrupt, endpoint, data, timeout)
            .await
    }
    pub fn device(&self) -> Device {
        self.handle.device()
    }
}
