#![allow(clippy::derive_partial_eq_without_eq)]

mod builders;
mod composite_type;
mod convert;
mod error;
mod extensions;
mod field;
mod field_selection;
mod fields;
mod index;
mod internal_data_model;
mod internal_enum;
mod model;
mod order_by;
mod parent_container;
mod prisma_value_ext;
mod projections;
mod record;
mod relation;
mod selection_result;

pub mod pk;
pub mod prelude;

pub use composite_type::*;
pub use convert::convert;
pub use dml;
pub use error::*;
pub use field::*;
pub use field_selection::*;
pub use fields::*;
pub use index::*;
pub use internal_data_model::*;
pub use internal_enum::*;
pub use model::*;
pub use order_by::*;
pub use prisma_value_ext::*;
pub use projections::*;
pub use record::*;
pub use relation::*;
pub use selection_result::*;

// Re-exports
pub use prisma_value::*;
pub use psl;

pub type Result<T> = std::result::Result<T, DomainError>;
