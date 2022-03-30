//! This module contains shared constants and logic that can be used by engines.

pub mod preview_features;
pub mod provider_names;

pub(crate) use datamodel_connector::constraint_names;

mod name_normalizer;
mod relation_names;

pub use relation_names::RelationNames;
