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
pub use database::{activate_driver_adapter, Js};
pub use error::SqlError;

#[cfg(not(target_arch = "wasm32"))]
pub use database::{Mssql, Mysql, PostgreSql, Sqlite};

type Result<T> = std::result::Result<T, error::SqlError>;
