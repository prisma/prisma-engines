//! Preview feature handling for the migration engine

use datamodel::Configuration;
use enumflags2::BitFlags;

/// Parse features from data model configuration.
pub fn from_config(_config: &Configuration) -> BitFlags<MigrationFeature> {
    BitFlags::empty()
}

#[derive(BitFlags, Copy, Clone, Debug, PartialEq)]
/// Feature flags to enable in the migration engine
pub enum MigrationFeature {}
