#![deny(warnings)]

#[macro_use]
extern crate debug_stub_derive;

mod datamodel_converter;
mod error;
mod field;
mod fields;
mod index;
mod internal_data_model;
mod model;
mod order_by;
mod prisma_args;
mod prisma_value_ext;
mod record;
mod relation;
mod selected_fields;

#[cfg(feature = "sql-ext")]
pub mod sql_ext;

pub mod prelude;

pub use datamodel::dml;
pub use datamodel_converter::*;
pub use error::*;
pub use field::*;
pub use fields::*;
pub use index::*;
pub use internal_data_model::*;
pub use model::*;
pub use order_by::*;
pub use prisma_args::*;
pub use prisma_args::*;
pub use prisma_value_ext::*;
pub use record::*;
pub use relation::*;
pub use selected_fields::*;

// reexport
pub use prisma_value::*;

#[cfg(feature = "sql-ext")]
pub use sql_ext::*;

pub type DomainResult<T> = Result<T, DomainError>;
