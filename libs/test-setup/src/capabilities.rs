use enumflags2::BitFlags;

/// Test-relevant connector capabilities.
#[derive(BitFlags, Copy, Clone, Debug, PartialEq)]
#[repr(u8)]
pub enum Capabilities {
    ScalarLists = 0b0001,
    Enums = 0b0010,
    Json = 0b0100,
    CreateDatabase = 0b1000,
}
