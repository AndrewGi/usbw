//! USB Semvar versioning.
#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash)]
pub struct Version(pub u16);
impl From<u16> for Version {
    fn from(i: u16) -> Self {
        Version(i)
    }
}
impl From<Version> for u16 {
    fn from(v: Version) -> Self {
        v.0
    }
}
impl Version {
    /// Creates a new USB BCD (Binary Coded Decimal) Version in the format `A.B.C`, where `A` is
    /// the major, `B` is the minor, and `C` is the sub minor. `A` is 8 bits while `B` and `C` are
    /// only `4 bits`.
    /// # Panics
    /// Panics if `minor > 0x0F_u8 || sub_minor > 0x0F_u8`
    pub fn new(major: u8, minor: u8, sub_minor: u8) -> Version {
        assert!(
            minor <= 0x0F_u8 && sub_minor < 0x0F_u8,
            "minor or sub_minor greater than 0x0F"
        );
        Version(u16::from(major) << 8 | u16::from(minor << 4) | u16::from(sub_minor))
    }
    pub const fn major(self) -> u8 {
        ((self.0 & 0xFF00_u16) >> 8) as u8
    }
    pub const fn minor(self) -> u8 {
        ((self.0 & 0x00F0_u16) >> 4) as u8
    }
    pub const fn sub_minor(self) -> u8 {
        (self.0 & 0x000F_u16) as u8
    }
}
