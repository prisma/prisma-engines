/// Test-relevant connector capabilities.
#[enumflags2::bitflags]
#[derive(Copy, Clone, Debug, PartialEq)]
#[repr(u8)]
pub enum Capabilities {
    ScalarLists = 1,
    Enums = 1 << 1,
    Json = 1 << 2,
    CreateDatabase = 1 << 3,
    LogicalReplication = 1 << 4,
}
