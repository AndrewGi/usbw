use crate::libusb::transfer::Transfer;

pub struct Pool {
    transfers: Vec<Transfer>,
}
impl Pool {
    pub fn pop_transfer(&mut self) -> Transfer {
        // TODO: iso packets
        self.transfers.pop().unwrap_or_else(|| Transfer::new(0))
    }
    pub fn push_transfer(&mut self, transfer: Transfer) {
        self.transfers.push(transfer)
    }
}
