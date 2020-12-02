use crate::libusb::device::Device;
use crate::libusb::device_handle::DeviceHandle;
use crate::libusb::error::Error;
use crate::libusb::safe_transfer::{SafeTransfer, SafeTransferAsyncLink};
use crate::libusb::transfer::{ControlSetup, Transfer, TransferType};
use std::convert::TryInto;

/// The Synchronous libusb interface converted to rust async. Warning, each function will
/// allocate a `Transfer` and a buffer for any data + `ControlSetup::SIZE`.
pub struct AsyncDevice {
    pub(crate) handle: DeviceHandle,
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
impl From<BulkType> for TransferType {
    fn from(b: BulkType) -> Self {
        b.transfer_type()
    }
}
impl AsyncDevice {
    /// # Safety
    /// Will block if a `AsyncContext` is running with the device's context
    pub unsafe fn from_device(handle: DeviceHandle) -> AsyncDevice {
        AsyncDevice { handle }
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
        })?;
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
        let mut transfer = SafeTransfer::from_buf(vec![0_u8; data.len() + ControlSetup::SIZE]);
        transfer.set_timeout(timeout);
        transfer.control_data_mut()[..data.len()].copy_from_slice(data);
        // Fill transfer with control parameters
        transfer.set_control_setup(ControlSetup {
            request_type,
            request,
            value,
            index,
            len: data.len().try_into().expect("too much data"),
        })?;
        transfer.submit_write(self).await
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
        &self,
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

struct InactiveTransfer {
    buf: Vec<u8>,
    transfer: Transfer,
    link: SafeTransferAsyncLink,
}
impl InactiveTransfer {
    fn safe_transfer<TempBuf>(
        &mut self,
        buf: TempBuf,
    ) -> SafeTransfer<TempBuf, &mut Transfer, &mut SafeTransferAsyncLink> {
        SafeTransfer::from_parts(buf, &mut self.transfer, &mut self.link)
    }
    fn control_transfer(
        &mut self,
        data: &[u8],
        setup: ControlSetup,
    ) -> SafeTransfer<&mut [u8], &mut Transfer, &mut SafeTransferAsyncLink> {
        self.buf.resize(data.len() + ControlSetup::SIZE, 0_u8);
        setup.serialize(self.buf.as_mut_slice());
        self.buf.as_mut_slice()[ControlSetup::SIZE..].copy_from_slice(data);
        SafeTransfer::from_parts(self.buf.as_mut_slice(), &mut self.transfer, &mut self.link)
    }
}

/// A [`AsyncDevice`] but reusing a `Vec<u8>` underneath to save allocations. While
/// [`SafeTransfer`]s are thread-safe, this struct has the use the safe buffer for all transfers
/// so a `&mut self` is required for all IO functions on this struct.
pub struct SingleTransferDevice {
    device: AsyncDevice,
    transfer: InactiveTransfer,
}
impl SingleTransferDevice {
    pub const fn from_parts(
        device: AsyncDevice,
        transfer: Transfer,
        buf: Vec<u8>,
        link: SafeTransferAsyncLink,
    ) -> Self {
        Self {
            device,
            transfer: InactiveTransfer {
                buf,
                transfer,
                link,
            },
        }
    }
    pub fn buf_clear(&mut self) {
        self.transfer.buf.clear();
    }
    pub fn buf_len(&self) -> usize {
        self.transfer.buf.len()
    }
    pub fn buf_reserve(&mut self, length_to_reserve: usize) {
        self.transfer.buf.reserve(length_to_reserve)
    }
    fn check_if_control_data_fits(&self, data_len: usize) -> Result<(), Error> {
        if self.buf_len() < ControlSetup::SIZE + data_len {
            Err(Error::Overflow)
        } else {
            Ok(())
        }
    }
    pub fn new(device: AsyncDevice) -> Self {
        Self::from_parts(
            device,
            Transfer::new(0),
            Vec::new(),
            SafeTransferAsyncLink::new(),
        )
    }
    pub async fn control_read(
        &mut self,
        request_type: u8,
        request: u8,
        value: u16,
        index: u16,
        data: &mut [u8],
        timeout: core::time::Duration,
    ) -> Result<usize, Error> {
        let mut transfer = self.transfer.control_transfer(
            &[],
            ControlSetup {
                request_type,
                request,
                value,
                index,
                len: data.len().try_into().expect("too much data"),
            },
        );
        transfer.set_timeout(timeout);
        let len = transfer.submit_write(&self.device).await?;
        data[..len].copy_from_slice(&transfer.control_data_ref()[..len]);
        Ok(len)
    }
    pub async fn control_write(
        &mut self,
        request_type: u8,
        request: u8,
        value: u16,
        index: u16,
        data: &[u8],
        timeout: core::time::Duration,
    ) -> Result<usize, Error> {
        let mut transfer = self.transfer.control_transfer(
            data,
            ControlSetup {
                request_type,
                request,
                value,
                index,
                len: data.len().try_into().expect("too much data"),
            },
        );
        transfer.set_timeout(timeout);
        // Fill transfer with control parameters
        transfer.submit_write(&self.device).await
    }
    pub async fn bulk_type_write(
        &mut self,
        bulk_type: BulkType,
        endpoint: u8,
        data: &[u8],
        timeout: core::time::Duration,
    ) -> Result<usize, Error> {
        let mut transfer = self.transfer.safe_transfer(data);
        transfer.set_type(bulk_type.into());
        transfer.set_endpoint(endpoint);
        transfer.set_timeout(timeout);
        transfer.submit_write(&self.device).await
    }

    pub async fn bulk_type_read(
        &mut self,
        bulk_type: BulkType,
        endpoint: u8,
        data: &mut [u8],
        timeout: core::time::Duration,
    ) -> Result<usize, Error> {
        let mut transfer = self.transfer.safe_transfer(data);
        transfer.set_type(bulk_type.into());
        transfer.set_endpoint(endpoint);
        transfer.set_timeout(timeout);
        transfer.submit_read(&self.device).await
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
}
