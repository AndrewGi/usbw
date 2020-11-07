use crate::libusb::device::Device;
use crate::libusb::device_handle::DeviceHandle;
use crate::libusb::error::Error;
use crate::libusb::transfer::{ControlSetup, Transfer, TransferType, TransferWithBuf};
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
    /// SAFETY: Will block if a `AsyncContext` is running with the device's context
    pub unsafe fn from_device(handle: DeviceHandle) -> AsyncDevice {
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
        if request_type & libusb1_sys::constants::LIBUSB_ENDPOINT_DIR_MASK
            != libusb1_sys::constants::LIBUSB_ENDPOINT_IN
        {
            return Err(Error::InvalidParam);
        }
        // Allocate Transfer
        let mut transfer = Transfer::new(0);
        // Allocate buffer for data (have to allocate data.len() + ControlSetup::SIZE sadly)
        let mut buf = vec![0; data.len() + ControlSetup::SIZE].into_boxed_slice();
        // Allocate CallbackData that enables Async
        let (tx, completed_wait) = oneshot::channel();
        let mut callback = Box::new(CallbackData::new(tx));
        // Fill transfer with control parameters
        transfer.clear_flags();
        transfer.set_timeout(timeout);
        transfer.set_callback(Self::system_callback);
        transfer.set_user_data(&mut callback as &mut CallbackData as *mut CallbackData);
        let mut transfer = TransferWithBuf::new(transfer, buf.as_mut());
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
        let len = transfer.transfer_ref().try_actual_length()? as usize;
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
        if request_type & libusb1_sys::constants::LIBUSB_ENDPOINT_DIR_MASK
            != libusb1_sys::constants::LIBUSB_ENDPOINT_OUT
        {
            return Err(Error::InvalidParam);
        }
        // Allocate Transfer
        let mut transfer = Transfer::new(0);
        // Allocate buffer for data (have to allocate data.len() + ControlSetup::SIZE sadly)
        let mut buf = vec![0; data.len() + ControlSetup::SIZE].into_boxed_slice();
        // Allocate CallbackData that enables Async
        let (tx, completed_wait) = oneshot::channel();
        let mut callback = Box::new(CallbackData::new(tx));
        // Set transfer parameters
        transfer.clear_flags();
        transfer.set_timeout(timeout);
        transfer.set_callback(Self::system_callback);
        transfer.set_user_data(&mut callback as &mut CallbackData as *mut CallbackData);
        let mut transfer = TransferWithBuf::new(transfer, buf.as_mut());

        // Fill with write data
        transfer.control_data_mut().copy_from_slice(data);
        // Fill transfer with control parameters
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
        let len = transfer.transfer_ref().try_actual_length()? as usize;
        Ok(len)
    }
    pub async fn bulk_type_write(
        &self,
        bulk_type: BulkType,
        endpoint: u8,
        data: &[u8],
        timeout: core::time::Duration,
    ) -> Result<usize, Error> {
        if endpoint & libusb1_sys::constants::LIBUSB_ENDPOINT_DIR_MASK
            != libusb1_sys::constants::LIBUSB_ENDPOINT_OUT
        {
            return Err(Error::InvalidParam);
        }
        // Allocate Transfer
        let mut transfer = Transfer::new(0);
        // Allocate CallbackData that enables Async
        let (tx, completed_wait) = oneshot::channel();
        let mut callback = Box::new(CallbackData::new(tx));
        // Set transfer parameters
        transfer.clear_flags();
        transfer.set_timeout(timeout);
        transfer.set_type(bulk_type.transfer_type());
        transfer.set_endpoint(endpoint);
        transfer.set_device(&self.handle);
        transfer.set_callback(Self::system_callback);
        transfer.set_user_data(&mut callback as &mut CallbackData as *mut CallbackData);
        // Set buffer
        transfer.set_buffer(data.as_ptr() as *mut _, data.len());

        // Send the transfer off
        unsafe { transfer.submit() }?;
        // TODO: Check if sender is dropped
        completed_wait
            .await
            .expect("sender was dropped, Andrew need to fix this");
        let len = transfer.try_actual_length()? as usize;
        Ok(len)
    }

    pub async fn bulk_type_read(
        &self,
        bulk_type: BulkType,
        endpoint: u8,
        data: &mut [u8],
        timeout: core::time::Duration,
    ) -> Result<usize, Error> {
        if endpoint & libusb1_sys::constants::LIBUSB_ENDPOINT_DIR_MASK
            != libusb1_sys::constants::LIBUSB_ENDPOINT_IN
        {
            return Err(Error::InvalidParam);
        }
        // Allocate Transfer
        let mut transfer = Transfer::new(0);
        // Allocate CallbackData that enables Async
        let (tx, completed_wait) = oneshot::channel();
        let mut callback = Box::new(CallbackData::new(tx));
        // Set transfer parameters
        transfer.clear_flags();
        transfer.set_timeout(timeout);
        transfer.set_type(bulk_type.transfer_type());
        transfer.set_endpoint(endpoint);
        transfer.set_device(&self.handle);
        transfer.set_callback(Self::system_callback);
        transfer.set_user_data(&mut callback as &mut CallbackData as *mut CallbackData);
        // Set buffer
        let transfer = TransferWithBuf::new(transfer, data);

        // Send the transfer off
        unsafe { transfer.transfer_ref().submit() }?;
        // TODO: Check if sender is dropped
        completed_wait
            .await
            .expect("sender was dropped, Andrew need to fix this");
        let len = transfer.transfer_ref().try_actual_length()? as usize;
        Ok(len)
    }
    pub async fn bulk_write(
        &mut self,
        endpoint: u8,
        data: &[u8],
        timeout: core::time::Duration,
    ) -> Result<usize, Error> {
        self.bulk_type_write(BulkType::Bulk, endpoint, data, timeout)
            .await
    }
    pub async fn interrupt_write(
        &mut self,
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
        &mut self,
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
