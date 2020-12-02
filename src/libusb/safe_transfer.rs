use crate::libusb::async_device::AsyncDevice;
use crate::libusb::error::Error;
use crate::libusb::transfer::{ControlSetup, Flags, Transfer, TransferType};
use core::borrow::BorrowMut;
use core::mem;
use core::sync::atomic::{AtomicBool, Ordering};
use driver_async::asyncs::sync::mpsc;
use driver_async::asyncs::task::block_on_future;

struct UserData {
    sender: mpsc::Sender<()>,
    is_active: AtomicBool,
}

impl UserData {
    pub fn send_completion(&self) {
        debug_assert_eq!(self.is_active.load(Ordering::SeqCst), true);
        self.is_active.store(false, Ordering::SeqCst);
        // Ignore if receiver is dropped
        self.sender.try_send(()).ok();
    }
}

pub struct SafeTransferAsyncLink {
    receiver: mpsc::Receiver<()>,
    user_data: Box<UserData>,
}

impl SafeTransferAsyncLink {
    pub fn new() -> Self {
        let (sender, receiver) = mpsc::channel(1);
        SafeTransferAsyncLink {
            receiver,
            user_data: Box::new(UserData {
                sender,
                is_active: AtomicBool::new(false),
            }),
        }
    }
}

pub struct SafeTransfer<
    Buf,
    Trans: BorrowMut<Transfer> = Transfer,
    Link: BorrowMut<SafeTransferAsyncLink> = SafeTransferAsyncLink,
> {
    buf: Buf,
    transfer: Trans,
    link: Link,
}

impl<Buf, Trans: BorrowMut<Transfer>, Link: BorrowMut<SafeTransferAsyncLink>>
    SafeTransfer<Buf, Trans, Link>
{
    pub fn from_parts(buf: Buf, transfer: Trans, link: Link) -> Self {
        Self {
            buf,
            transfer,
            link,
        }
    }
}
impl<Buf> SafeTransfer<Buf, Transfer, SafeTransferAsyncLink> {
    pub fn from_buf(buf: Buf) -> Self {
        Self::from_transfer_buf(Transfer::new(0), buf)
    }
    pub fn from_transfer_buf(transfer: Transfer, buf: Buf) -> Self {
        Self::from_parts(buf, transfer, SafeTransferAsyncLink::new())
    }
}
impl<Buf, Trans: BorrowMut<Transfer>, Link: BorrowMut<SafeTransferAsyncLink>>
    SafeTransfer<Buf, Trans, Link>
{
    extern "system" fn system_callback(transfer: *mut libusb1_sys::libusb_transfer) {
        let mut transfer = unsafe {
            Transfer::from_libusb(
                core::ptr::NonNull::new(transfer).expect("null transfer ptr in callback"),
            )
        };
        Self::callback(&mut transfer);
        // Forget because dropping call's `libusb_transfer_free` and that is handled elsewhere
        core::mem::forget(transfer)
    }
    fn callback(transfer: &mut Transfer) {
        if transfer.libusb_ref().user_data.is_null() {
            return;
        }
        let user_data = unsafe { transfer.cast_userdata_ref::<UserData>() };
        // Signal completion
        user_data.send_completion();
    }
    pub fn is_active(&self) -> bool {
        self.link
            .borrow()
            .user_data
            .is_active
            .load(Ordering::SeqCst)
    }
    pub async fn into_parts(mut self) -> (Buf, Trans, Link) {
        self.wait_for_inactive().await;
        self.into_all_parts()
    }
    /// # Safety
    /// Must be called when inactive
    fn into_all_parts(mut self) -> (Buf, Trans, Link) {
        debug_assert!(
            !self.is_active(),
            "deconstructing SafeTransfer while active"
        );
        // # Safety
        // Manual dropping of the fields in order to move `buf` and `transfer` out of the struct
        unsafe {
            let buf = (&mut self.buf as *mut Buf).read();
            let transfer = (&mut self.transfer as *mut Trans).read();
            let link = (&mut self.link as *mut Link).read();
            mem::forget(self);
            (buf, transfer, link)
        }
    }
    pub async fn into_buf(self) -> Buf {
        self.into_parts().await.0
    }
    pub async fn into_transfer(self) -> Trans {
        self.into_parts().await.1
    }
    async fn wait_for_receiver(&mut self) {
        self.link.borrow_mut().receiver.recv().await;
    }
    async fn wait_for_inactive(&mut self) {
        if self.is_active() {
            self.wait_for_receiver().await
        }
    }
    fn sync_wait_for_cancel(&mut self) -> Result<(), Error> {
        if self.cancel_asynchronously()? {
            block_on_future(self.wait_for_inactive())
        }
        Ok(())
    }
    fn check_endpoint(&self, is_read: bool) -> Result<(), Error> {
        if self.transfer_ref().is_endpoint_read() != is_read {
            Err(Error::InvalidParam)
        } else {
            Ok(())
        }
    }
    pub fn set_timeout(&mut self, timeout: core::time::Duration) {
        self.transfer.borrow_mut().set_timeout(timeout)
    }
    pub fn get_timeout(&self) -> core::time::Duration {
        self.transfer_ref().get_timeout()
    }
    pub fn get_endpoint(&self) -> u8 {
        self.transfer_ref().get_endpoint()
    }
    pub fn set_endpoint(&mut self, endpoint: u8) {
        self.transfer.borrow_mut().set_endpoint(endpoint)
    }

    fn set_active(&self, is_active: bool) {
        self.link
            .borrow()
            .user_data
            .is_active
            .store(is_active, Ordering::SeqCst)
    }
    pub fn get_type(&self) -> TransferType {
        self.transfer_ref().get_type()
    }
    pub fn set_type(&mut self, transfer_type: TransferType) {
        self.transfer.borrow_mut().set_type(transfer_type)
    }
    fn ensure_inactive(&self) -> Result<(), Error> {
        if self.is_active() {
            Err(Error::Busy)
        } else {
            Ok(())
        }
    }
    pub fn transfer_ref(&self) -> &Transfer {
        self.transfer.borrow()
    }
    /// Returns if it did try to cancel
    fn cancel_asynchronously(&self) -> Result<bool, Error> {
        if self.is_active() {
            unsafe { self.transfer_ref().cancel() }?;
            Ok(true)
        } else {
            Ok(false)
        }
    }
    pub fn buf_ref(&self) -> &Buf {
        &self.buf
    }
    pub fn buf_mut(&mut self) -> &mut Buf {
        &mut self.buf
    }
}

impl<Buf, Trans: BorrowMut<Transfer>, Link: BorrowMut<SafeTransferAsyncLink>> Drop
    for SafeTransfer<Buf, Trans, Link>
{
    fn drop(&mut self) {
        self.sync_wait_for_cancel()
            .expect("SafeTransfer drop cancel failed. This should never happen")
    }
}

impl<Buf: AsRef<[u8]>, Trans: BorrowMut<Transfer>, Link: BorrowMut<SafeTransferAsyncLink>>
    SafeTransfer<Buf, Trans, Link>
{
    /// # Safety
    /// This fills the transfer with information including pointers. This function is safe to call
    /// as long as you make sure the `Buf` is mutable vs immutable ('BorrowMut' vs 'Borrow') because
    /// `libusb` might write data
    fn set_fields(&mut self) {
        let buf = self.buf.as_ref();
        let trans = self.transfer.borrow_mut();
        trans.set_buffer(buf.as_ptr() as *mut u8, buf.len());
        trans.set_flags(Flags::ZEROED);
        trans.set_callback(Self::system_callback);
        trans.set_user_data(&mut *self.link.borrow_mut().user_data as *mut UserData);
    }
    fn get_control_setup(&self) -> Option<ControlSetup> {
        let buf = self.buf.as_ref();
        if buf.len() > ControlSetup::SIZE {
            Some(ControlSetup::deserialize(buf))
        } else {
            None
        }
    }
    fn check_control_setup(&self, is_read: bool) -> Result<(), Error> {
        // TODO: Check endpoint?
        self.ensure_inactive()?;
        let control_setup = self.try_control_setup()?;
        if control_setup.is_read() != is_read {
            return Err(Error::InvalidParam);
        }
        if usize::from(control_setup.len) <= self.calculated_control_data_len() {
            Ok(())
        } else {
            Err(Error::Overflow)
        }
    }
    pub async fn submit_write(&mut self, device_handle: &AsyncDevice) -> Result<usize, Error> {
        self.submit(device_handle, false).await
    }
    pub fn control_data_ref(&self) -> &[u8] {
        &self.buf.as_ref()[ControlSetup::SIZE..]
    }
    pub fn calculated_control_data_len(&self) -> usize {
        self.buf.as_ref().len().saturating_sub(ControlSetup::SIZE)
    }
    fn try_control_setup(&self) -> Result<ControlSetup, Error> {
        self.get_control_setup().ok_or(Error::NotFound)
    }
    pub fn control_setup_len_field(&self) -> Result<u16, Error> {
        self.try_control_setup().map(|c| c.len)
    }
    fn check_transfer(&self, is_read: bool) -> Result<(), Error> {
        match self.transfer.borrow().get_type() {
            TransferType::Control => self.check_control_setup(is_read),
            TransferType::Bulk | TransferType::Interrupt => self.check_endpoint(is_read),
            TransferType::Stream => unimplemented!("libusb stream are not yet implemented"),
            TransferType::Isochronous => {
                unimplemented!("libusb isochronous are not yet implemented")
            }
        }
    }
    fn submit_asynchronously(&self, is_read: bool) -> Result<(), Error> {
        self.check_transfer(is_read)?;
        self.set_active(true);
        // Send the transfer off
        match unsafe { self.transfer.borrow().submit() } {
            Ok(_) => Ok(()),
            Err(e) => {
                // ensure its set to inactive
                self.set_active(false);
                Err(e)
            }
        }
    }
    async fn submit(&mut self, device_handle: &AsyncDevice, is_read: bool) -> Result<usize, Error> {
        self.set_fields();
        self.transfer
            .borrow_mut()
            .set_device(device_handle.handle_ref());

        // Submit
        self.submit_asynchronously(is_read)?;
        // Wait for completion
        self.wait_for_inactive().await;
        // Set to inactive
        debug_assert_eq!(self.is_active(), false, "transfer still active");
        // Return actual data transferred length
        self.transfer
            .borrow()
            .try_actual_length()
            .map(|l| l as usize)
    }
}
impl<
        Buf: AsMut<[u8]> + AsRef<[u8]>,
        Trans: BorrowMut<Transfer>,
        Link: BorrowMut<SafeTransferAsyncLink>,
    > SafeTransfer<Buf, Trans, Link>
{
    pub fn set_control_setup(&mut self, control_setup: ControlSetup) -> Result<(), Error> {
        let buf = self.buf.as_mut();
        if buf.len() > ControlSetup::SIZE {
            control_setup.serialize(buf);
            Ok(())
        } else {
            Err(Error::Overflow)
        }
    }
    pub fn control_data_mut(&mut self) -> &mut [u8] {
        &mut self.buf.as_mut()[ControlSetup::SIZE..]
    }

    pub async fn submit_read(&mut self, device_handle: &AsyncDevice) -> Result<usize, Error> {
        self.submit(device_handle, true).await
    }
}
