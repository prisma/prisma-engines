#![allow(
    clippy::wrong_self_convention,
    clippy::branches_sharing_code,
    clippy::needless_borrow,
    clippy::needless_collect
)]
#![deny(unsafe_code)]

mod column_metadata;
mod context;
mod cursor_condition;
mod database;
mod error;
mod filter_conversion;
mod join_utils;
mod model_extensions;
mod nested_aggregations;
mod ordering;
mod query_arguments_ext;
mod query_builder;
mod query_ext;
mod row;
mod sql_info;
mod sql_trace;
mod value;
mod value_ext;

use self::{column_metadata::*, context::Context, filter_conversion::*, query_ext::QueryExt, row::*};

pub use database::{FromSource, Mssql, Mysql, PostgreSql, Sqlite};
pub use error::SqlError;

type Result<T> = std::result::Result<T, error::SqlError>;
