use bitflags::bitflags;

bitflags! {
    pub struct Capabilities: u8 {
        const SCALAR_LISTS = 0b00000001;
        const ENUMS        = 0b00000010;
    }
}

#[derive(Debug)]
pub struct UnknownCapabilityError(String);

impl std::fmt::Display for UnknownCapabilityError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Unknown capability `{}`.", self.0)
    }
}

impl std::error::Error for UnknownCapabilityError {}

impl std::str::FromStr for Capabilities {
    type Err = UnknownCapabilityError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "scalar_lists" => Ok(Capabilities::SCALAR_LISTS),
            "enums" => Ok(Capabilities::ENUMS),
            _ => Err(UnknownCapabilityError(s.to_owned())),
        }
    }
}
