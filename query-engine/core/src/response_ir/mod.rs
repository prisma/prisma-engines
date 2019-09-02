//! Prisma Response IR (Intermediate Representation).
//!
//! This module takes care of processing the results
//! and transforming them into a different AST.
//!
//! This IR is meant for general processing and storage.
//! It can also be easily serialized.
//!
//! The code itself can be considered WIP. It is clear when reading the code that there are missing abstractions
//! and a restructure might be necessary (good example is the default value handling sprinkled all over the place).
mod read;
mod utils;
mod write;

pub use read::*;
pub use utils::*;
pub use write::*;

use crate::{schema::OutputType, ExpressionResult, ResultInfo};
use indexmap::IndexMap;
use prisma_models::PrismaValue;
use std::{borrow::Borrow, sync::Arc};

/// A `key -> value` map to an IR item
pub type Map = IndexMap<String, Item>;

/// A list of IR items
pub type List = Vec<Item>;

/// Convenience type wrapper for Arc<Item>.
pub type ItemRef = Arc<Item>;

/// A response can either be some `key-value` data representation
/// or an error that occured.
#[derive(Debug)]
pub enum Response {
    Data(String, Item),
    Error(String),
}

// todo merge of responses

/// An IR item that either expands to a subtype or leaf-record.
#[derive(Debug, Clone)]
pub enum Item {
    Map(Map),
    List(List),
    Value(PrismaValue),

    /// Wrapper type to allow multiple parent records
    /// to claim the same item without copying data
    /// (serialization can then choose how to copy if necessary).
    Ref(ItemRef),
}

/// An IR builder utility
/// TODO ordering of top level results?
#[derive(Debug, Default)]
pub struct ResultIrBuilder;

impl ResultIrBuilder {
    pub fn build(result: ExpressionResult, info: ResultInfo) -> Response {
        match result {
            ExpressionResult::Read(r) => {
                match serialize_read(r, &info.output_type, false, false) {
                    Ok(result) => {
                        // On the top level, each result pair boils down to a exactly a single serialized result.
                        // All checks for lists and optionals have already been performed during the recursion,
                        // so we just unpack the only result possible.
                        // Todo: The following checks feel out of place. Imo this needs to be handled already one level deeper.
                        let result = if result.is_empty() {
                            match info.output_type.borrow() {
                                OutputType::Opt(_) => Item::Value(PrismaValue::Null),
                                OutputType::List(_) => Item::List(vec![]),
                                _ => unreachable!(),
                            }
                        } else {
                            let (_, item) = result.into_iter().take(1).next().unwrap();
                            item
                        };

                        Response::Data(info.key, result)
                    }
                    Err(err) => Response::Error(format!("{}", err)),
                }
            }

            ExpressionResult::Write(w) => {
                let serialized = serialize_write(w, &info);

                match serialized {
                    Ok(result) => Response::Data(info.key, result),
                    Err(err) => Response::Error(format!("{}", err)),
                }
            }
        }
    }
}
