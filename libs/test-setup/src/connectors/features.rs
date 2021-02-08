use enumflags2::BitFlags;
use once_cell::sync::Lazy;

#[derive(BitFlags, Copy, Clone, Debug, PartialEq)]
#[repr(u8)]
pub enum Features {
    Other = 0b1,
}

impl Features {
    pub fn empty() -> BitFlags<Features> {
        BitFlags::empty()
    }

    pub fn from_name(name: &str) -> Result<BitFlags<Features>, UnknownFeatureError> {
        FEATURE_NAMES
            .binary_search_by_key(&name, |(name, _feature)| *name)
            .ok()
            .and_then(|idx| FEATURE_NAMES.get(idx))
            .map(|(_name, feature)| *feature)
            .ok_or_else(|| UnknownFeatureError(name.to_owned()))
    }
}

#[derive(Debug)]
pub struct UnknownFeatureError(String);

impl std::fmt::Display for UnknownFeatureError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let available_features: Vec<&str> = FEATURE_NAMES.iter().map(|(name, _)| *name).collect();
        write!(
            f,
            "Unknown feature `{}`. Available features: {:?}",
            self.0, available_features
        )
    }
}

impl std::error::Error for UnknownFeatureError {}

/// All the features, sorted by name.
static FEATURE_NAMES: Lazy<Vec<(&str, BitFlags<Features>)>> = Lazy::new(Vec::new);
