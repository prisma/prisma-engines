//! # quaint
//!
//! A database client abstraction for reading and writing to an SQL database in a
//! safe manner.
//!
//! ### Goals
//!
//! - Query generation when the database and conditions are not known at compile
//!   time.
//! - Parameterized queries.
//! - A modular design, separate AST for query building and visitors for
//!   different databases.
//! - Database support behind a feature flag.
//!
//! ### Non-goals
//!
//! - Database-level type-safety in query building or being an ORM.
//!
//! ## Databases
//!
//! - SQLite
//! - PostgreSQL
//! - MySQL
//! - Microsoft SQL Server
//!
//! ### Methods of connecting
//!
//! Quaint provides two options to connect to the underlying database.
//!
//! The [single connection method](single/struct.Quaint.html):
//!
//! ``` rust
//! use quaint::{prelude::*, single::Quaint};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), quaint::error::Error> {
//!     let conn = Quaint::new("file:///tmp/example.db").await?;
//!     let result = conn.select(Select::default().value(1)).await?;
//!
//!     assert_eq!(
//!         Some(1),
//!         result.into_iter().nth(0).and_then(|row| row[0].as_i64()),
//!     );
//!
//!     Ok(())
//! }
//! ```
//!
//! The [pooled method](pooled/struct.Quaint.html):
//!
//! ``` rust
//! use quaint::{prelude::*, pooled::Quaint};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), quaint::error::Error> {
//!     let pool = Quaint::builder("file:///tmp/example.db")?.build();
//!     let conn = pool.check_out().await?;
//!     let result = conn.select(Select::default().value(1)).await?;
//!
//!     assert_eq!(
//!         Some(1),
//!         result.into_iter().nth(0).and_then(|row| row[0].as_i64()),
//!     );
//!
//!     Ok(())
//! }
//! ```
//!
//! ### Using the AST module
//!
//! The crate can be used as an SQL string builder using the [ast](ast/index.html) and
//! [visitor](visitor/index.html) modules.
//!
//! AST is generic for all databases and the visitors generate correct SQL
//! syntax for the database.
//!
//! The visitor returns the query as a string and its parameters as a vector.
//!
//! ```
//! # use quaint::{prelude::*, visitor::{Sqlite, Visitor}};
//! # fn main() -> Result<(), quaint::error::Error> {
//! let conditions = "word"
//!     .equals("meow")
//!     .and("age".less_than(10))
//!     .and("paw".equals("warm"));
//!
//! let query = Select::from_table("naukio").so_that(conditions);
//! let (sql_str, params) = Sqlite::build(query)?;
//!
//! assert_eq!(
//!     "SELECT `naukio`.* FROM `naukio` WHERE (`word` = ? AND `age` < ? AND `paw` = ?)",
//!     sql_str,
//! );
//!
//! assert_eq!(
//!     vec![
//!         Value::from("meow"),
//!         Value::from(10),
//!         Value::from("warm"),
//!     ],
//!     params
//! );
//! # Ok(())
//! # }
//! ```

#![cfg_attr(feature = "docs", feature(doc_cfg))]

#[cfg(not(any(feature = "sqlite", feature = "postgresql", feature = "mysql", feature = "mssql")))]
compile_error!("one of 'sqlite', 'postgresql', 'mysql' or 'mssql' features must be enabled");

#[macro_use]
mod macros;

#[macro_use]
extern crate metrics;

#[cfg(feature = "bigdecimal")]
extern crate bigdecimal_ as bigdecimal;

pub mod ast;
pub mod connector;
pub mod error;
#[cfg(feature = "pooled")]
#[cfg_attr(feature = "docs", doc(cfg(pooled)))]
pub mod pooled;
pub mod prelude;
#[cfg(feature = "serde-support")]
#[cfg_attr(feature = "docs", doc(cfg(feature = "serde-support")))]
pub mod serde;
pub mod single;
#[cfg(test)]
mod tests;
pub mod visitor;

use once_cell::sync::Lazy;

pub use ast::Value;

pub(crate) static LOG_QUERIES: Lazy<bool> = Lazy::new(|| std::env::var("LOG_QUERIES").map(|_| true).unwrap_or(false));

pub type Result<T> = std::result::Result<T, error::Error>;
