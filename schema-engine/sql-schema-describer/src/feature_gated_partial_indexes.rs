use serde::{Deserialize, Serialize};
use std::{
    collections::HashSet,
    fmt,
    ops::{Deref, DerefMut},
};

use crate::IndexId;

/// A helper for tracking feature-gated partial indexes in SqlSchema.
#[derive(Default, Clone)]
pub(crate) struct FeatureGatedPartialIndexes(HashSet<IndexId>);

impl fmt::Debug for FeatureGatedPartialIndexes {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("<FeatureGatedPartialIndexes>")
    }
}

impl Serialize for FeatureGatedPartialIndexes {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_unit()
    }
}

impl<'de> Deserialize<'de> for FeatureGatedPartialIndexes {
    fn deserialize<D>(_deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        Ok(Default::default())
    }
}

impl Deref for FeatureGatedPartialIndexes {
    type Target = HashSet<IndexId>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for FeatureGatedPartialIndexes {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
