#![deny(warnings)]

#[macro_use]
extern crate debug_stub_derive;

mod datamodel_converter;
mod enum_type;
mod error;
mod field;
mod fields;
mod index;
mod internal_data_model;
mod model;
mod order_by;
mod prisma_value;
mod record;
mod relation;
mod selected_fields;

#[cfg(feature = "sql-ext")]
pub mod sql_ext;

pub mod prelude;

pub use datamodel::dml;
pub use datamodel::DataSourceField;
pub use datamodel_converter::*;
pub use enum_type::*;
pub use error::*;
pub use field::*;
pub use fields::*;
pub use index::*;
pub use internal_data_model::*;
pub use model::*;
pub use order_by::*;
pub use prisma_value::*;
pub use record::*;
pub use relation::*;
pub use selected_fields::*;

#[cfg(feature = "sql-ext")]
pub use sql_ext::*;

pub type Result<T> = std::result::Result<T, DomainError>;
