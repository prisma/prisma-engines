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
        let available_capability_names: Vec<&str> = CAPABILITY_NAMES.iter().map(|(name, _)| *name).collect();

        write!(
            f,
            "Unknown capability `{}`. Available capabilities: {:?}",
            self.0, available_capability_names
        )
    }
}

impl std::error::Error for UnknownCapabilityError {}

impl std::str::FromStr for Capabilities {
    type Err = UnknownCapabilityError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        CAPABILITY_NAMES
            .binary_search_by_key(&s, |(name, _capability)| *name)
            .ok()
            .and_then(|idx| CAPABILITY_NAMES.get(idx))
            .map(|(_name, capability)| *capability)
            .ok_or_else(|| UnknownCapabilityError(s.to_owned()))
    }
}

/// All the capabilities, sorted by name.
const CAPABILITY_NAMES: &[(&str, Capabilities)] = &[
    ("enums", Capabilities::ENUMS),
    ("scalar_lists", Capabilities::SCALAR_LISTS),
];
