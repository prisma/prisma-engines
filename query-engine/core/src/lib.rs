#![warn(warnings)] // todo deny warnings once done

// #[macro_use]
// extern crate log;

#[macro_use]
extern crate debug_stub_derive;

#[macro_use]
extern crate lazy_static;

pub mod error;
pub mod executor;
pub mod query_builders;
pub mod query_document;
pub mod query_graph;
pub mod response_ir;
pub mod schema;
pub mod schema_builder;

pub use query_graph::*;
pub use query_builders::*;
pub use response_ir::*;
pub use schema_builder::*;
pub use error::*;
pub use executor::*;
pub use schema::*;
pub use query_document::*;

use schema::OutputTypeRef;

pub type CoreResult<T> = Result<T, CoreError>;

/// WIP Holds all necessary meta info to serialize a result.
pub struct ResultInfo {
    pub key: String,
    pub output_type: OutputTypeRef,
    pub selected_fields: Vec<String>, // Temporary workaround to hold state for write queries
    // query args?
}

// /// Temporary type.
// /// Purely a workaround to not mess with the internals of the write query and result ASTs for now.
// /// Reason: We need the name information of the query for serialization purposes.
// #[derive(Debug, Clone)]
// pub struct WriteQueryResultWrapper {
//     pub name: String,
//     pub alias: Option<String>,
//     pub result: WriteQueryResult,
// }
