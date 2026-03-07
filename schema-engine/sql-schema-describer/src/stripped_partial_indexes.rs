use serde::{Deserialize, Serialize};
use std::{
    collections::HashSet,
    fmt,
    ops::{Deref, DerefMut},
};

use crate::IndexId;

/// A helper for tracking stripped partial indexes in SqlSchema.
#[derive(Default, Clone)]
pub(crate) struct StrippedPartialIndexes(HashSet<IndexId>);

impl fmt::Debug for StrippedPartialIndexes {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("<StrippedPartialIndexes>")
    }
}

impl Serialize for StrippedPartialIndexes {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_unit()
    }
}

impl<'de> Deserialize<'de> for StrippedPartialIndexes {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        <serde::de::IgnoredAny as Deserialize>::deserialize(deserializer)?;
        Ok(Default::default())
    }
}

impl Deref for StrippedPartialIndexes {
    type Target = HashSet<IndexId>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for StrippedPartialIndexes {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
