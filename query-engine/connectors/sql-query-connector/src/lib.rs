#![allow(clippy::wrong_self_convention)]
#![deny(unsafe_code)]

mod column_metadata;
pub mod context;
mod cursor_condition;
mod database;
mod error;
mod filter;
mod join_utils;
mod limit;
pub mod model_extensions;
mod nested_aggregations;
mod ordering;
pub mod query_arguments_ext;
pub mod query_builder;
mod query_ext;
mod row;
mod ser_raw;
mod sql_trace;
mod value;

use self::{column_metadata::*, context::Context, query_ext::QueryExt, row::*};
use quaint::prelude::Queryable;

pub use database::operations::write::generate_insert_statements;

pub use database::FromSource;
#[cfg(feature = "driver-adapters")]
pub use database::Js;
pub use error::SqlError;

#[cfg(feature = "mssql-native")]
pub use database::Mssql;

#[cfg(feature = "mysql-native")]
pub use database::Mysql;

#[cfg(feature = "postgresql-native")]
pub use database::PostgreSql;

#[cfg(feature = "sqlite-native")]
pub use database::Sqlite;

type Result<T> = std::result::Result<T, error::SqlError>;
