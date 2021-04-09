//! This module contains shared constants and logic that can be used by engines.
//!
mod default_names;
mod name_normalizer;

pub mod preview_features;
pub mod provider_names;

pub use default_names::RelationNames;
pub use name_normalizer::NameNormalizer;
