use enumflags2::BitFlags;

#[derive(BitFlags, Copy, Clone, Debug, PartialEq)]
#[repr(u8)]
pub enum Capabilities {
    ScalarLists = 0b00001,
    Enums = 0b00010,
    Json = 0b00100,
    CreateDatabase = 0b01000,
    Decimal = 0b10000,
}
