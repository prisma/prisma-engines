//! The feature handling for SQL Migration Connector.

use std::{fmt::Display, io, str::FromStr};

use datamodel::Configuration;
use enumflags2::BitFlags;

static NATIVE_TYPES: &str = "nativeTypes";

/// Parse features from data model configuration.
pub fn from_config(config: &Configuration) -> BitFlags<MigrationFeature> {
    config.preview_features().fold(BitFlags::empty(), |mut acc, feature| {
        let feature: io::Result<MigrationFeature> = feature.parse();

        match feature {
            Ok(feature) => acc.insert(feature),
            Err(e) => tracing::debug!("{}", e),
        }

        acc
    })
}

#[derive(BitFlags, Copy, Clone, Debug, PartialEq)]
#[repr(u8)]
/// Feature flags to enable in Migration Engine
pub enum MigrationFeature {
    /// Use native types in diffing and migrating.
    NativeTypes = 0b00000001,
}

impl FromStr for MigrationFeature {
    type Err = io::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            s if s == NATIVE_TYPES => Ok(Self::NativeTypes),
            _ => {
                let kind = io::ErrorKind::InvalidInput;

                Err(io::Error::new(
                    kind,
                    format!("Native type `{}` not supported in Migration Engine.", s),
                ))
            }
        }
    }
}

impl Display for MigrationFeature {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NativeTypes => write!(f, "{}", NATIVE_TYPES),
        }
    }
}
