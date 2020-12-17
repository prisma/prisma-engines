//! The feature handling for SQL Migration Connector.

use std::{io, str::FromStr};

use datamodel::Configuration;
use enumflags2::BitFlags;

/// Parse features from data model configuration.
pub fn from_config(config: &Configuration) -> BitFlags<SqlFeature> {
    config.preview_features().fold(BitFlags::empty(), |mut acc, feature| {
        let feature: io::Result<SqlFeature> = feature.parse();

        match feature {
            Ok(feature) => acc.insert(feature),
            Err(e) => tracing::debug!("{}", e),
        }

        acc
    })
}

#[derive(BitFlags, Copy, Clone, Debug, PartialEq)]
#[repr(u8)]
/// Sql feature flags to enable in Migration Engine
pub enum SqlFeature {
    /// Use native types in diffing and migrating.
    NativeTypes = 0b00000001,
}

impl FromStr for SqlFeature {
    type Err = io::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "nativeTypes" => Ok(Self::NativeTypes),
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
