#![allow(clippy::wrong_self_convention)]
#![deny(unsafe_code)]

mod column_metadata;
mod context;
mod cursor_condition;
mod database;
mod error;
mod filter;
mod join_utils;
mod model_extensions;
mod nested_aggregations;
mod ordering;
mod query_arguments_ext;
mod query_builder;
mod query_ext;
mod row;
mod sql_trace;
mod value;
mod value_ext;

use self::{column_metadata::*, context::Context, query_ext::QueryExt, row::*};
use quaint::prelude::Queryable;

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
