use serde::{Deserialize, Serialize};
use std::fmt;

use crate::cloneable_any::CloneableAny;

/// A helper for arbitrary connector-specific data in SqlSchema.
#[derive(Default)]
pub struct ConnectorData {
    pub data: Option<Box<dyn CloneableAny + Send + Sync>>,
}

impl PartialEq for ConnectorData {
    fn eq(&self, _other: &Self) -> bool {
        true
    }
}

impl fmt::Debug for ConnectorData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("<ConnectorData>")
    }
}

impl Serialize for ConnectorData {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_unit()
    }
}

impl<'de> Deserialize<'de> for ConnectorData {
    fn deserialize<D>(_deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        Ok(Default::default())
    }
}

impl Clone for ConnectorData {
    fn clone(&self) -> Self {
        let cloned_data = self.data.as_ref().map(|d| (&**d).clone_box());

        ConnectorData { data: cloned_data }
    }
}
