#![allow(unused_unsafe)]
use crate::libusb::async_device::AsyncDevice;
use crate::libusb::device_handle::DeviceHandle;
use crate::libusb::error::Error;
use core::convert::TryFrom;
use core::convert::TryInto;
use core::mem;
use core::sync::atomic::{AtomicBool, Ordering};
use driver_async::asyncs::sync::mpsc;
use driver_async::asyncs::task::block_on_future;

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Debug, Hash)]
pub enum Status {
    Completed = 0,
    Error = 1,
    TimedOut = 2,
    Cancelled = 3,
    Stall = 4,
    NoDevice = 5,
    Overflow = 6,
}
impl Status {
    pub fn from_i32(i: i32) -> Option<Status> {
        match i {
            0 => Some(Status::Completed),
            1 => Some(Status::Error),
            2 => Some(Status::TimedOut),
            3 => Some(Status::Cancelled),
            4 => Some(Status::Stall),
            5 => Some(Status::NoDevice),
            6 => Some(Status::Overflow),
            _ => None,
        }
    }
}
impl Status {
    pub fn as_error(self) -> Result<(), Error> {
        match self {
            Status::Completed => Ok(()),
            Status::Error | Status::Cancelled => Err(Error::Io),
            Status::TimedOut => Err(Error::Timeout),
            Status::Stall => Err(Error::Pipe),
            Status::NoDevice => Err(Error::NoDevice),
            Status::Overflow => Err(Error::Overflow),
        }
    }
}
impl From<Status> for i32 {
    fn from(s: Status) -> Self {
        s as i32
    }
}
impl TryFrom<i32> for Status {
    type Error = ();

    fn try_from(value: i32) -> Result<Self, ()> {
        Self::from_i32(value).ok_or(())
    }
}
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Debug, Hash)]
pub enum TransferType {
    Control = 0,
    Isochronous = 1,
    Bulk = 2,
    Interrupt = 3,
    Stream = 4,
}
impl From<TransferType> for u8 {
    fn from(t: TransferType) -> Self {
        t as u8
    }
}
impl TryFrom<u8> for TransferType {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(TransferType::Control),
            1 => Ok(TransferType::Isochronous),
            2 => Ok(TransferType::Bulk),
            3 => Ok(TransferType::Interrupt),
            4 => Ok(TransferType::Stream),
            _ => Err(()),
        }
    }
}
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Debug, Hash)]
pub enum Flag {
    ShortNotOk = 0,
    FreeBuffer = 1,
    FreeTransfer = 2,
    AddZeroPacket = 3,
}
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Debug, Hash, Default)]
pub struct Flags(u8);
impl Flags {
    pub const ZEROED: Flags = Flags::new(0);
    pub const fn new(flags: u8) -> Flags {
        // TODO: Maybe clear higher bits?
        Flags(flags)
    }
    pub const fn inner(self) -> u8 {
        self.0
    }
    pub fn get(self, flag: Flag) -> bool {
        self.0 & (1_u8 << (flag as u8)) != 0
    }
    pub fn set(&mut self, flag: Flag) {
        self.0 |= 1_u8 << (flag as u8)
    }
    pub fn clear(&mut self, flag: Flag) {
        self.0 &= !(1_u8 << (flag as u8))
    }
}
impl From<Flags> for u8 {
    fn from(f: Flags) -> Self {
        f.inner()
    }
}
impl From<u8> for Flags {
    fn from(u: u8) -> Self {
        Flags::new(u)
    }
}
/// Any Serialization or deserialization of this struct should be careful to make sure the `u16`s
/// are in Little Endian for the wire and Host Endian at all other times.
#[derive(Copy, Clone, Debug)]
#[repr(C)]
pub struct ControlSetup {
    pub request_type: u8,
    pub request: u8,
    pub value: u16,
    pub index: u16,
    pub len: u16,
}
impl ControlSetup {
    pub const SIZE: usize = core::mem::size_of::<Self>();
    /// Taste Host-Endian `ControlSetup` and serializes it in Little-Endian
    pub fn serialize(self, buf: &mut [u8]) {
        assert!(buf.len() >= Self::SIZE, "ControlSetup buf too small");
        let le = ControlSetup {
            request_type: self.request_type,
            request: self.request,
            value: self.value.to_le(),
            index: self.index.to_le(),
            len: self.len.to_le(),
        };
        // Unaligned write because `buf` is only 1-byte aligned and `ControlSetup`
        // might need aligned
        unsafe { core::ptr::write_unaligned(buf.as_mut_ptr() as *mut Self, le) }
    }
    pub fn deserialize(buf: &[u8]) -> ControlSetup {
        assert!(buf.len() >= Self::SIZE, "ControlSetup buf too small");
        let le = unsafe { core::ptr::read_unaligned(buf.as_ptr() as *const Self) };
        ControlSetup {
            request_type: le.request_type,
            request: le.request,
            value: u16::from_le(le.value),
            index: u16::from_le(le.index),
            len: u16::from_le(le.len),
        }
    }
    pub fn is_write(&self) -> bool {
        self.request_type & libusb1_sys::constants::LIBUSB_ENDPOINT_DIR_MASK
            == libusb1_sys::constants::LIBUSB_ENDPOINT_OUT
    }
    pub fn is_read(&self) -> bool {
        self.request_type & libusb1_sys::constants::LIBUSB_ENDPOINT_DIR_MASK
            == libusb1_sys::constants::LIBUSB_ENDPOINT_IN
    }
}
/// [`Transfer`] tries to be a lightweight safe abstraction over [`libusb1_sys::libusb_transfer`].
/// Only a limited subset of actions are safe on the libusb_transfer. Stuff like setting the data
/// pointer are unsafe or should be abstracted over (like `SafeTransfer`).
#[derive(Debug)]
pub struct Transfer(core::ptr::NonNull<libusb1_sys::libusb_transfer>);
impl Transfer {
    pub fn new(iso_packets: usize) -> Transfer {
        Transfer(
            core::ptr::NonNull::new(unsafe {
                libusb1_sys::libusb_alloc_transfer(iso_packets as i32)
            })
            .expect("null libusb transfer ptr"),
        )
    }
    /// Allows access to the inner  [`libusb1_sys::libusb_transfer`] internals.
    pub fn libusb_inner(&self) -> core::ptr::NonNull<libusb1_sys::libusb_transfer> {
        self.0
    }
    pub fn libusb_ref(&self) -> &libusb1_sys::libusb_transfer {
        unsafe { self.0.as_ref() }
    }
    pub fn clear_flags(&mut self) {
        self.libusb_mut().flags = 0;
    }
    pub fn set_device(&mut self, device: &DeviceHandle) {
        self.libusb_mut().dev_handle = device.inner().as_ptr();
    }
    pub fn fill_control(&mut self, device: &DeviceHandle) {
        let inner = self.libusb_mut();
        inner.transfer_type = TransferType::Control.into();
        inner.endpoint = 0;
        inner.num_iso_packets = 0;
        inner.dev_handle = device.inner().as_ptr();
    }
    pub fn set_num_iso_packets(&mut self, num: usize) {
        self.libusb_mut().num_iso_packets = num as i32;
    }
    pub fn is_endpoint_read(&self) -> bool {
        self.libusb_ref().endpoint & libusb1_sys::constants::LIBUSB_ENDPOINT_DIR_MASK
            != libusb1_sys::constants::LIBUSB_ENDPOINT_OUT
    }
    pub fn get_num_iso_packets(&self) -> usize {
        self.libusb_ref().num_iso_packets as usize
    }
    pub fn set_callback(&mut self, new_callback: libusb1_sys::libusb_transfer_cb_fn) {
        self.libusb_mut().callback = new_callback
    }
    pub fn get_type(&self) -> TransferType {
        self.libusb_ref()
            .transfer_type
            .try_into()
            .expect("invalid transfer type")
    }
    pub fn set_type(&mut self, transfer_type: TransferType) {
        self.libusb_mut().transfer_type = transfer_type.into();
    }
    pub fn get_callback(&self) -> libusb1_sys::libusb_transfer_cb_fn {
        self.libusb_ref().callback
    }
    pub fn set_endpoint(&mut self, new_endpoint: u8) {
        self.libusb_mut().endpoint = new_endpoint;
    }
    pub fn get_endpoint(&self) -> u8 {
        self.libusb_ref().endpoint
    }
    /// Checks `.status()` to make sure its `Status::Completed` before returning `Ok(actual_length)`.
    /// If `.status()` is not `Status::Completed`, it will return a `Err(status_error)`
    pub fn try_actual_length(&self) -> Result<i32, Error> {
        match self.status() {
            Some(status) => match status {
                Status::Completed => Ok(self.actual_length()),
                Status::Error | Status::Cancelled => Err(Error::Io),
                Status::TimedOut => Err(Error::Timeout),
                Status::Stall => Err(Error::Pipe),
                Status::NoDevice => Err(Error::NoDevice),
                Status::Overflow => Err(Error::Overflow),
            },
            None => Err(Error::Other),
        }
    }
    pub fn actual_length(&self) -> i32 {
        self.libusb_ref().actual_length
    }
    pub fn libusb_mut(&mut self) -> &mut libusb1_sys::libusb_transfer {
        unsafe { self.0.as_mut() }
    }
    pub fn set_buffer(&mut self, buffer: *mut u8, len: usize) {
        self.libusb_mut().buffer = buffer;
        self.libusb_mut().length = len as i32;
    }
    /// # Safety
    /// The transfer status and pointers could cause memory to be read and write. Memory Safety
    /// isn't guaranteed for this struct
    pub unsafe fn submit(&self) -> Result<(), Error> {
        try_unsafe!(libusb1_sys::libusb_submit_transfer(self.0.as_ptr()));
        Ok(())
    }
    /// # Safety
    /// The transfer status and pointers could cause memory to be read and write. Memory Safety
    /// isn't guaranteed for this struct
    pub unsafe fn cancel(&self) -> Result<(), Error> {
        try_unsafe!(libusb1_sys::libusb_cancel_transfer(self.0.as_ptr()));
        Ok(())
    }
    pub fn get_flags(&self) -> Flags {
        self.libusb_ref().flags.into()
    }
    pub fn set_flags(&mut self, new_flags: Flags) {
        self.libusb_mut().flags = new_flags.inner()
    }
    pub fn set_stream_id(&mut self, id: u32) {
        unsafe { libusb1_sys::libusb_transfer_set_stream_id(self.0.as_ptr(), id) }
    }
    pub fn get_stream_id(&self) -> u32 {
        unsafe { libusb1_sys::libusb_transfer_get_stream_id(self.0.as_ptr()) }
    }
    /// # Safety
    /// Treats the pointer as a reference and it could dereference dangling memory
    pub unsafe fn from_libusb(ptr: core::ptr::NonNull<libusb1_sys::libusb_transfer>) -> Transfer {
        Transfer(ptr)
    }
    pub fn set_timeout(&mut self, timeout: core::time::Duration) {
        self.libusb_mut().timeout = timeout.as_millis().try_into().unwrap_or(u32::MAX)
    }
    pub fn get_timeout(&self) -> core::time::Duration {
        core::time::Duration::from_millis(self.libusb_ref().timeout.try_into().unwrap_or(0_u64))
    }
    pub fn status(&self) -> Option<Status> {
        self.libusb_ref().status.try_into().ok()
    }
    pub fn set_user_data<T>(&mut self, user_data: *mut T) {
        self.libusb_mut().user_data = user_data as *mut _
    }
    /// # Safety
    /// Casting a void pointer to any type
    pub unsafe fn cast_userdata_ref<T>(&self) -> &T {
        unsafe { &*(self.libusb_ref().user_data as *const T) }
    }
    /// # Safety
    /// Casting a void pointer to any type
    pub unsafe fn cast_userdata_mut<T>(&mut self) -> &mut T {
        unsafe { &mut *(self.libusb_ref().user_data as *mut T) }
    }
}
impl Drop for Transfer {
    fn drop(&mut self) {
        unsafe { libusb1_sys::libusb_free_transfer(self.0.as_ptr()) }
    }
}

pub struct TransferWithBuf<'transfer, 'buf> {
    transfer_buf: &'buf mut [u8],
    transfer: &'transfer mut Transfer,
}
impl<'t, 'b> TransferWithBuf<'t, 'b> {
    /// WARNING! The `transfer_buf` holds more than just the data to be read/sent
    pub fn new(transfer: &'t mut Transfer, transfer_buf: &'b mut [u8]) -> Self {
        transfer.set_buffer(transfer_buf.as_mut_ptr(), transfer_buf.len());
        Self {
            transfer_buf,
            transfer,
        }
    }
    /// Returns the old `transfer_buf`
    pub fn set_buf(&mut self, new_buf: &'b mut [u8]) -> &'b mut [u8] {
        self.transfer
            .set_buffer(new_buf.as_mut_ptr(), new_buf.len());
        core::mem::replace(&mut self.transfer_buf, new_buf)
    }
    pub fn buf_mut(&mut self) -> &mut [u8] {
        self.transfer_buf
    }
    pub fn buf_ref(&self) -> &[u8] {
        self.transfer_buf
    }
    pub fn transfer_ref(&self) -> &Transfer {
        &self.transfer
    }
    pub unsafe fn transfer_mut_unsafe(&mut self) -> &mut Transfer {
        &mut self.transfer
    }
    pub(crate) fn transfer_mut(&mut self) -> &mut Transfer {
        &mut self.transfer
    }
    pub fn control_data_ref(&self) -> &[u8] {
        &self.transfer_buf[ControlSetup::SIZE..]
    }
    pub fn control_data_mut(&mut self) -> &mut [u8] {
        &mut self.transfer_buf[ControlSetup::SIZE..]
    }
    pub fn set_control_setup(&mut self, handle: &DeviceHandle, control_setup: ControlSetup) {
        assert!(
            self.transfer_buf.len() >= ControlSetup::SIZE,
            "buf smaller than a ControlSetup, maybe missing it?"
        );
        control_setup.serialize(self.transfer_buf);
        self.transfer.fill_control(handle);
    }
}
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
pub struct SafeTransfer<Buf> {
    buf: Buf,
    transfer: Transfer,
    receiver: mpsc::Receiver<()>,
    user_data: Box<UserData>,
}
impl<Buf> SafeTransfer<Buf> {
    pub fn from_parts(transfer: Transfer, buf: Buf) -> Self {
        let (sender, receiver) = mpsc::channel(1);
        Self {
            buf,
            transfer,
            receiver,
            user_data: Box::new(UserData {
                sender,
                is_active: AtomicBool::new(false),
            }),
        }
    }
    pub fn from_buf(buf: Buf) -> Self {
        Self::from_parts(buf, Transfer::new(0))
    }
    extern "system" fn system_callback(transfer: *mut libusb1_sys::libusb_transfer) {
        println!("callback");
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
        self.user_data.is_active.load(Ordering::SeqCst)
    }
    pub async fn into_parts(mut self) -> (Buf, Transfer) {
        self.wait_for_inactive().await;
        // # Safety
        // Manual dropping of the fields in order to move `buf` and `transfer` out of the struct
        unsafe {
            let buf = (&mut self.buf as *mut Buf).read();
            let transfer = (&mut self.transfer as *mut Transfer).read();
            drop((&mut self.user_data as *mut Box<UserData>).read());
            drop((&mut self.receiver as *mut mpsc::Receiver<()>).read());
            mem::forget(self);
            (buf, transfer)
        }
    }
    pub async fn into_buf(self) -> Buf {
        self.into_parts().await.0
    }
    pub async fn into_transfer(self) -> Transfer {
        self.into_parts().await.1
    }
    async fn wait_for_receiver(&mut self) {
        self.receiver.recv().await;
    }
    async fn wait_for_inactive(&mut self) {
        if self.is_active() {
            self.wait_for_receiver().await
        }
    }
    fn sync_wait_for_cancel(&mut self) {
        if self.is_active() {
            block_on_future(self.wait_for_inactive())
        }
    }
    fn check_endpoint(&self, is_read: bool) -> Result<(), Error> {
        if self.transfer.is_endpoint_read() != is_read {
            Err(Error::InvalidParam)
        } else {
            Ok(())
        }
    }
    pub fn set_timeout(&mut self, timeout: core::time::Duration) {
        self.transfer.set_timeout(timeout)
    }
    pub fn get_timeout(&self) -> core::time::Duration {
        self.transfer.get_timeout()
    }
    pub fn get_endpoint(&self) -> u8 {
        self.transfer.get_endpoint()
    }
    pub fn set_endpoint(&mut self, endpoint: u8) {
        self.transfer.set_endpoint(endpoint)
    }
    fn set_fields(&mut self) {
        let buf = self.buf.as_mut();
        self.transfer.set_buffer(buf.as_mut_ptr(), buf.len());
        self.transfer.set_flags(Flags::ZEROED);
        self.transfer.set_callback(Self::system_callback);
        self.transfer.set_user_data(&mut self.user_data as *mut _);
    }

    fn calculated_control_data_len(&self) -> usize {
        self.buf.as_ref().len().saturating_sub(ControlSetup::SIZE)
    }
    fn set_active(&self, is_active: bool) {
        self.user_data.is_active.store(is_active, Ordering::SeqCst)
    }
    pub fn get_type(&self) -> TransferType {
        self.transfer.get_type()
    }
    pub fn set_type(&mut self, transfer_type: TransferType) {
        self.transfer.set_type(transfer_type)
    }
    fn ensure_inactive(&self) -> Result<(), Error> {
        if self.is_active() {
            Err(Error::Busy)
        } else {
            Ok(())
        }
    }
}
impl<Buf> Drop for SafeTransfer<Buf> {
    fn drop(&mut self) {
        self.sync_wait_for_cancel()
    }
}

impl<Buf: AsMut<[u8]> + AsRef<[u8]>> SafeTransfer<Buf> {
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
        self.submit(device_handle, true)
    }
}

impl<Buf: AsRef<[u8]>> SafeTransfer<Buf> {
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
    pub fn control_data_ref(&self) -> &[u8] {
        &self.buf.as_ref()[ControlSetup::SIZE..]
    }
    fn try_control_setup(&self) -> Result<ControlSetup, Error> {
        self.get_control_setup().ok_or(Error::NotFound)
    }
    pub fn control_setup_len_field(&self) -> Result<u16, Error> {
        self.try_control_setup().map(|c| c.len)
    }
    pub async fn submit_write(&mut self, device_handle: &AsyncDevice) -> Result<usize, Error> {
        self.submit(device_handle, false)
    }
    fn check_transfer(&self, is_read: bool) -> Result<(), Error> {
        match self.transfer.get_type() {
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
        match unsafe { self.transfer.submit() } {
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
        self.transfer.set_device(device_handle.handle_ref());

        // Submit
        self.submit_asynchronously(is_read)?;
        // Wait for completion
        self.wait_for_inactive().await;
        // Set to inactive
        debug_assert_eq!(self.is_active(), false, "transfer still active");
        self.set_active(false);
        // Return actual data transferred length
        self.transfer.try_actual_length().map(|l| l as usize)
    }
}
