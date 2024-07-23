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

pub use response::*;

pub(crate) use ir_serializer::*;

use crate::ArgumentValue;
use indexmap::IndexMap;
use query_structure::{PrismaValue, RawJson};
use serde::ser::{Serialize, SerializeMap, SerializeSeq, Serializer};
use std::{collections::HashMap, fmt, sync::Arc};

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

    pub fn index_by(self, keys: &[String]) -> Vec<(HashMap<String, ArgumentValue>, Map)> {
        let mut map: Vec<(HashMap<String, ArgumentValue>, Map)> = Vec::with_capacity(self.len());

        for item in self.into_iter() {
            let inner = item.into_map().unwrap();
            let key: HashMap<String, ArgumentValue> = keys
                .iter()
                .map(|key| {
                    let item = inner.get(key).unwrap().clone();
                    let pv = item.into_value().unwrap();

                    (key.clone(), pv)
                })
                .map(|(key, val)| (key, ArgumentValue::from(val)))
                .collect();

            map.push((key, inner));
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
    RawJson(RawJson),

    /// Wrapper type to allow multiple parent records
    /// to claim the same item without copying data
    /// (serialization can then choose how to copy if necessary).
    Ref(ItemRef),
}

impl Item {
    pub fn null() -> Self {
        Self::Value(PrismaValue::Null)
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

    /// Returns a mutable reference to the underlying map, if the element is a map and the map is
    /// owned. Unlike [`Item::as_map`], it doesn't allow obtaining a reference to a shared map
    /// referenced via [`ItemRef`].
    pub fn as_map_mut(&mut self) -> Option<&mut Map> {
        match self {
            Self::Map(m) => Some(m),
            _ => None,
        }
    }

    pub fn into_map(self) -> Option<Map> {
        match self {
            Self::Map(m) => Some(m),
            Self::Ref(r) => Arc::try_unwrap(r).ok().and_then(|r| r.into_map()),
            _ => None,
        }
    }

    pub fn into_value(self) -> Option<PrismaValue> {
        match self {
            Self::Value(pv) => Some(pv),
            Self::Ref(r) => Arc::try_unwrap(r).ok().and_then(|r| r.into_value()),
            Self::List(list) => {
                let mut values = vec![];

                for item in list {
                    if let Some(pv) = item.into_value() {
                        values.push(pv)
                    } else {
                        return None;
                    }
                }

                Some(PrismaValue::List(values))
            }
            _ => None,
        }
    }

    pub fn into_list(self) -> Option<List> {
        match self {
            Self::List(l) => Some(l),
            Self::Ref(r) => Arc::try_unwrap(r).ok().and_then(|r| r.into_list()),
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
            Self::RawJson(value) => value.serialize(serializer),
            Self::Ref(item_ref) => item_ref.serialize(serializer),
        }
    }
}
