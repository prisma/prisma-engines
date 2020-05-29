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

use crate::{CoreError, ExpressionResult, OutputType, OutputTypeRef, QueryResult, QueryValue};
use indexmap::IndexMap;
use internal::*;
use prisma_models::PrismaValue;
use serde::ser::{Serialize, SerializeMap, SerializeSeq, Serializer};
use std::{borrow::Borrow, fmt, sync::Arc};

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

#[derive(Debug, serde::Serialize, PartialEq)]
pub struct ResponseError {
    error: String,
    user_facing_error: user_facing_errors::Error,
}

impl From<user_facing_errors::Error> for ResponseError {
    fn from(err: user_facing_errors::Error) -> ResponseError {
        ResponseError {
            error: err.message().to_owned(),
            user_facing_error: err,
        }
    }
}

impl From<crate::error::CoreError> for ResponseError {
    fn from(err: crate::error::CoreError) -> ResponseError {
        ResponseError {
            error: format!("{}", err),
            user_facing_error: err.into(),
        }
    }
}

/// A response can either be some `key-value` data representation
/// or an error that occured.
#[derive(Debug)]
pub enum Response {
    Data(String, Item),
    Error(ResponseError),
}

#[derive(Debug, serde::Serialize, Default, PartialEq)]
pub struct Responses {
    #[serde(skip_serializing_if = "IndexMap::is_empty")]
    data: Map,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    errors: Vec<ResponseError>,
}

impl Responses {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            data: IndexMap::with_capacity(capacity),
            ..Default::default()
        }
    }

    pub fn insert_data(&mut self, key: impl Into<String>, item: Item) {
        self.data.insert(key.into(), item);
    }

    pub fn insert_error(&mut self, error: impl Into<ResponseError>) {
        self.errors.push(error.into());
    }

    pub fn get_data(&self, key: impl AsRef<str>) -> Option<&Item> {
        self.data.get(key.as_ref())
    }

    pub fn take_data(&mut self, key: impl AsRef<str>) -> Option<Item> {
        self.data.remove(key.as_ref())
    }
}

impl From<Map> for Responses {
    fn from(data: Map) -> Self {
        Self {
            data,
            ..Default::default()
        }
    }
}

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

#[derive(Debug)]
pub struct IrSerializer {
    /// Serialization key for root DataItem
    /// Note: This will change
    pub key: String,

    /// Output type describing the possible shape of the result
    pub output_type: OutputTypeRef,
}

impl IrSerializer {
    pub fn serialize(&self, result: ExpressionResult) -> Response {
        match result {
            ExpressionResult::Query(QueryResult::Json(json)) => Response::Data(self.key.clone(), Item::Json(json)),
            ExpressionResult::Query(r) => {
                match serialize_internal(r, &self.output_type, false, false) {
                    Ok(result) => {
                        // On the top level, each result boils down to a exactly a single serialized result.
                        // All checks for lists and optionals have already been performed during the recursion,
                        // so we just unpack the only result possible.
                        // Todo: The following checks feel out of place. This probably needs to be handled already one level deeper.
                        let result = if result.is_empty() {
                            match self.output_type.borrow() {
                                OutputType::Opt(_) => Item::Value(PrismaValue::Null),
                                OutputType::List(_) => Item::list(Vec::new()),
                                other => return Response::Error(ResponseError::from(CoreError::SerializationError(format!(
                                    "Invalid response data: the query result was required, but an empty {:?} was returned instead.",
                                    other
                                )))),
                            }
                        } else {
                            let (_, item) = result.into_iter().take(1).next().unwrap();
                            item
                        };

                        Response::Data(self.key.clone(), result)
                    }
                    Err(err) => Response::Error(err.into()),
                }
            }

            ExpressionResult::Empty => panic!("Domain logic error: Attempted to serialize empty result."),
            ExpressionResult::Computation(_) => panic!("Domain logic error: Attempted to serialize non-query result."),
        }
    }
}
