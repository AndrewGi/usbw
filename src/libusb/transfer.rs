use crate::libusb::device_handle::DeviceHandle;
use crate::libusb::error::Error;
use core::convert::TryInto;
use std::convert::TryFrom;

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
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Debug, Hash)]
pub struct Flags(u8);
impl Flags {
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
    pub fn serialize(mut self, buf: &mut [u8]) {
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
    pub unsafe fn submit(&self) -> Result<(), Error> {
        try_unsafe!(libusb1_sys::libusb_submit_transfer(self.0.as_ptr()));
        Ok(())
    }
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
    pub unsafe fn from_libusb(ptr: core::ptr::NonNull<libusb1_sys::libusb_transfer>) -> Transfer {
        Transfer(ptr)
    }
    pub fn set_timeout(&mut self, timeout: core::time::Duration) {
        self.libusb_mut().timeout = timeout.as_millis().try_into().unwrap_or(u32::MAX)
    }
    pub fn status(&self) -> Option<Status> {
        self.libusb_ref().status.try_into().ok()
    }
    pub fn set_user_data<T>(&mut self, user_data: *mut T) {
        self.libusb_mut().user_data = user_data as *mut _
    }
    pub unsafe fn cast_userdata_ref<T>(&self) -> &T {
        unsafe { &*(self.libusb_ref().user_data as *const T) }
    }
    pub unsafe fn cast_userdata_mut<T>(&mut self) -> &mut T {
        unsafe { &mut *(self.libusb_ref().user_data as *mut T) }
    }
}
impl Drop for Transfer {
    fn drop(&mut self) {
        unsafe { libusb1_sys::libusb_free_transfer(self.0.as_ptr()) }
    }
}

pub struct SafeTransfer<'a> {
    transfer_buf: &'a mut [u8],
    transfer: Transfer,
}
impl<'a> SafeTransfer<'a> {
    /// WARNING! The `transfer_buf` holds more than just the data to be read/sent
    pub fn new(mut transfer: Transfer, transfer_buf: &'a mut [u8]) -> Self {
        transfer.set_buffer(transfer_buf.as_mut_ptr(), transfer_buf.len());
        Self {
            transfer_buf,
            transfer,
        }
    }
    /// Returns the old `transfer_buf`
    pub fn set_buf(&mut self, new_buf: &'a mut [u8]) -> &'a mut [u8] {
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
    pub fn transfer_mut(&mut self) -> &mut Transfer {
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
