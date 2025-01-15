#![allow(clippy::wrong_self_convention)]
#![deny(unsafe_code)]

mod database;
mod error;
mod query_ext;
mod row;
mod ser_raw;
mod value;

use self::{query_ext::QueryExt, row::*};
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
