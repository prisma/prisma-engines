//! This module contains shared constants and logic that can be used by engines.

mod preview_features;
mod relation_names;

pub use self::{
    preview_features::{FeatureMap, PreviewFeature, ALL_PREVIEW_FEATURES},
    relation_names::RelationNames,
};
