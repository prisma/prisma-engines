#![deny(warnings)]

mod datamodel_converter;
mod error;
mod field;
mod fields;
mod index;
mod internal_data_model;
mod model;
mod order_by;
mod prisma_value_ext;
mod projections;
mod record;
mod relation;

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
pub use prisma_value_ext::*;
pub use projections::*;
pub use record::*;
pub use relation::*;

// reexport
pub use prisma_value::*;

#[cfg(feature = "sql-ext")]
pub use sql_ext::*;

pub type Result<T> = std::result::Result<T, DomainError>;
