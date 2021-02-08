use enumflags2::BitFlags;

#[derive(Debug, Clone, Copy, BitFlags, PartialEq)]
#[repr(u8)]
pub enum ValidationFeature {
    // Do not validate datasoure
    IgnoreDatasourceUrls = 0b0000_0001,
    // Make implicit relations explicit and all that.
    StandardizeModels = 0b0000_0010,
    // Make errors pretty.
    PrettifyErrors = 0b0000_0100,
}
