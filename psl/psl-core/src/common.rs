//! This module contains shared constants and logic that can be used by engines.

mod preview_features;

pub(crate) use self::preview_features::RenamedFeature;
pub use self::preview_features::{FeatureMapWithProvider, PreviewFeature, PreviewFeatures, ALL_PREVIEW_FEATURES};
