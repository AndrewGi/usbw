#![allow(unused)]
use crate::libusb::transfer::Transfer;
#[derive(Clone, Debug)]
struct Inner {}
impl Inner {
    unsafe fn alloc(&self, _len: usize) -> *mut u8 {
        unimplemented!()
    }
}
pub struct Pool {
    transfers: Vec<Transfer>,
}
#[derive(Debug)]
pub struct Allocation {
    ptr: *mut u8,
    len: usize,
}
impl Allocation {
    pub fn ptr(&mut self) -> *mut u8 {
        self.ptr
    }
    pub fn len(&self) -> usize {
        self.len
    }
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
}
impl Pool {
    fn layout(len: usize) -> core::alloc::Layout {
        unsafe { core::alloc::Layout::from_size_align_unchecked(len, 2) }
    }
    unsafe fn allocate(&mut self, len: usize) -> *mut u8 {
        alloc::alloc::alloc(core::alloc::Layout::from_size_align(len, 2).expect("bad alloc layout"))
    }
    unsafe fn deallocate(&mut self, ptr: *mut u8, len: usize) {
        alloc::alloc::dealloc(ptr, Self::layout(len))
    }
    pub fn pop_transfer(&mut self) -> Transfer {
        // TODO: iso packets
        self.transfers.pop().unwrap_or_else(|| Transfer::new(0))
    }
    pub fn push_transfer(&mut self, transfer: Transfer) {
        self.transfers.push(transfer)
    }
}
