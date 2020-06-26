//! Prisma Response IR (Intermediate Representation).
//!
//! This module takes care of processing query execution results
//! and transforming them into a different AST.
//!
//! This IR is meant for general processing and storage.
//! It can also be easily serialized.
//!
//! Note: The code itself can be considered WIP. It is clear when reading the code that there are missing abstractions
//! and a restructure might be necessary (good example is the default value handling sprinkled all over the place).
mod internal;
mod ir_serializer;
mod response;

use crate::QueryValue;
use indexmap::IndexMap;
use prisma_models::{PrismaValue, TypeHint};
use serde::ser::{Serialize, SerializeMap, SerializeSeq, Serializer};
use std::{fmt, sync::Arc};

pub use ir_serializer::*;
pub use response::*;

/// A `key -> value` map to an IR item
pub type Map = IndexMap<String, Item>;

#[derive(Clone, PartialEq)]
pub struct List {
    inner: Vec<Item>,
}

impl fmt::Debug for List {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.inner.fmt(f)
    }
}

impl List {
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn index_by(self, keys: &[String]) -> IndexMap<Vec<QueryValue>, Map> {
        let mut map = IndexMap::with_capacity(self.len());

        for item in self.into_iter() {
            let inner = item.into_map().unwrap();

            let key = keys
                .iter()
                .map(|key| inner.get(&key.to_string()).unwrap().clone().into_value().unwrap())
                .map(QueryValue::from)
                .collect();

            map.insert(key, inner);
        }

        map
    }
}

impl From<Vec<Item>> for List {
    fn from(inner: Vec<Item>) -> Self {
        Self { inner }
    }
}

impl IntoIterator for List {
    type Item = Item;
    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.inner.into_iter()
    }
}

impl<'a> IntoIterator for &'a List {
    type Item = &'a Item;
    type IntoIter = std::slice::Iter<'a, Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.inner.iter()
    }
}

/// Convenience type wrapper for Arc<Item>.
pub type ItemRef = Arc<Item>;

/// An IR item that either expands to a subtype or leaf-record.
#[derive(Debug, Clone, PartialEq)]
pub enum Item {
    Map(Map),
    List(List),
    Value(PrismaValue),
    Json(serde_json::Value),

    /// Wrapper type to allow multiple parent records
    /// to claim the same item without copying data
    /// (serialization can then choose how to copy if necessary).
    Ref(ItemRef),
}

impl Item {
    pub fn null() -> Self {
        Self::Value(PrismaValue::Null(TypeHint::Unknown))
    }

    pub fn list(inner: Vec<Item>) -> Self {
        Self::List(List::from(inner))
    }

    pub fn as_map(&self) -> Option<&Map> {
        match self {
            Self::Map(m) => Some(m),
            Self::Ref(r) => r.as_map(),
            _ => None,
        }
    }

    pub fn into_map(self) -> Option<Map> {
        match self {
            Self::Map(m) => Some(m),
            Self::Ref(r) => Arc::try_unwrap(r).ok().map(|r| r.into_map()).flatten(),
            _ => None,
        }
    }

    pub fn into_value(self) -> Option<PrismaValue> {
        match self {
            Self::Value(pv) => Some(pv),
            Self::Ref(r) => Arc::try_unwrap(r).ok().map(|r| r.into_value()).flatten(),
            _ => None,
        }
    }

    pub fn into_list(self) -> Option<List> {
        match self {
            Self::List(l) => Some(l),
            Self::Ref(r) => Arc::try_unwrap(r).ok().map(|r| r.into_list()).flatten(),
            _ => None,
        }
    }
}

impl Serialize for Item {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Self::Map(m) => {
                let mut map = serializer.serialize_map(Some(m.len()))?;

                for (k, v) in m {
                    map.serialize_entry(k, v)?;
                }

                map.end()
            }
            Self::List(l) => {
                let mut seq = serializer.serialize_seq(Some(l.len()))?;

                for e in l {
                    seq.serialize_element(e)?;
                }

                seq.end()
            }
            Self::Value(pv) => pv.serialize(serializer),
            Self::Json(value) => value.serialize(serializer),
            Self::Ref(item_ref) => item_ref.serialize(serializer),
        }
    }
}
