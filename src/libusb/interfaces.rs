pub const INTERFACES_MAX: u8 = 0xFF;
pub const INTERFACES_BYTE_LEN: usize = (INTERFACES_MAX as usize + 1) / 8;
#[derive(Debug, Default, Hash)]
pub struct ClaimedInterfaces([u8; INTERFACES_BYTE_LEN]);
impl ClaimedInterfaces {
    pub const DEFAULT: ClaimedInterfaces = ClaimedInterfaces([0_u8; INTERFACES_BYTE_LEN]);
    pub const fn new() -> ClaimedInterfaces {
        Self::DEFAULT
    }
    pub const fn byte_index(interface: u8) -> u8 {
        interface / 8
    }
    pub const fn bit_index(interface: u8) -> u8 {
        interface % 8
    }
    pub fn claim(&mut self, interface: u8) {
        self.0[Self::byte_index(interface) as usize] |= 1 << Self::bit_index(interface)
    }
    pub fn is_claimed(&self, interface: u8) -> bool {
        self.0[Self::byte_index(interface) as usize] & (1 << Self::bit_index(interface)) != 0
    }
    pub fn release(&mut self, interface: u8) {
        self.0[Self::byte_index(interface) as usize] &= !(1 << Self::bit_index(interface))
    }
    pub fn any_claimed(&self) -> bool {
        self.0.iter().any(|&i| i != 0)
    }
    pub fn none_claimed(&self) -> bool {
        self.0.iter().all(|&i| i == 0)
    }
}
impl Iterator for ClaimedInterfaces {
    type Item = u8;

    fn next(&mut self) -> Option<Self::Item> {
        let mut index: u8 = 0;
        for b in self.0.iter_mut() {
            if *b == 0 {
                if index >= 0xFF - 8 {
                    break;
                }
                index += 8;
                continue;
            }
            // Naive CLZ
            for i in (0..8).rev() {
                if *b > (1 << i) {
                    *b -= 1 << i;
                    return Some(index + i);
                }
            }
        }
        None
    }
}
#[cfg(test)]
mod tests {
    use crate::libusb::interfaces::ClaimedInterfaces;

    #[test]
    pub fn test_clean_claim_interfaces() {
        let c = ClaimedInterfaces::new();
        assert!(!c.any_claimed());
        assert!(c.none_claimed());
        assert!(!c.is_claimed(0));
        assert!(!c.is_claimed(255))
    }
    #[test]
    pub fn test_claimed_claim_interfaces() {
        let mut c = ClaimedInterfaces::new();
        c.claim(6);
        assert!(c.any_claimed());
        assert!(!c.none_claimed());
        assert!(!c.is_claimed(0));
        assert!(!c.is_claimed(255));
        assert!(!c.is_claimed(5));
        assert!(!c.is_claimed(7));
        assert!(c.is_claimed(6));
        assert_eq!(c.next(), Some(6));
        assert!(!c.is_claimed(6))
    }
}
