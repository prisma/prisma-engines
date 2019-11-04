//! # quaint
//!
//! Quaint is an AST and database-specific visitors for creating SQL
//! statements.
//!
//! Under construction and will go through several rounds of changes. Not meant
//! for production use in the current form.
//!
//! ### Goals
//!
//! - Query generation when the database and conditions are not known beforehand.
//! - Parameterized queries when possible.
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
//!
//! ## Examples
//!
//! ### Building an SQL query string
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
//! use quaint::{ast::*, visitor::{Sqlite, Visitor}};
//!
//! fn main() {
//!     let conditions = "word"
//!         .equals("meow")
//!         .and("age".less_than(10))
//!         .and("paw".equals("warm"));
//!
//!     let query = Select::from_table("naukio").so_that(conditions);
//!     let (sql_str, params) = Sqlite::build(query);
//!
//!     assert_eq!(
//!         "SELECT `naukio`.* FROM `naukio` WHERE ((`word` = ? AND `age` < ?) AND `paw` = ?)",
//!         sql_str,
//!     );
//!
//!     assert_eq!(
//!         vec![
//!             ParameterizedValue::from("meow"),
//!             ParameterizedValue::from(10),
//!             ParameterizedValue::from("warm"),
//!         ],
//!         params
//!     );
//! }
//! ```
//!
//! ### Querying a database with an AST object
//!
//! The [connector](connector/index.html) module abstracts a generic query interface over
//! different databases. It offers querying with the [ast](ast/index.html) module or
//! directly using raw strings.
//!
//! When querying with an ast object the queries are paremeterized
//! automatically.
//!
//! ```
//! use quaint::{ast::*, connector::*};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), quaint::error::Error> {
//!     let mut conn = Sqlite::new("test.db")?;
//!     let query = Select::default().value(1);
//!     let result = conn.query(query.into()).await?;
//!
//!     assert_eq!(
//!         Some(1),
//!         result.into_iter().nth(0).and_then(|row| row[0].as_i64()),
//!     );
//!
//!     Ok(())
//! }
//! ```
pub mod ast;
#[cfg(any(feature = "mysql", feature = "postgresql", feature = "sqlite"))]
pub mod connector;
pub mod error;

#[cfg(any(feature = "mysql", feature = "postgresql", feature = "sqlite"))]
pub mod pool;
pub mod visitor;

#[cfg(not(feature = "tracing-log"))]
#[macro_use]
extern crate log;

#[macro_use]
extern crate metrics;

#[macro_use]
extern crate debug_stub_derive;

pub type Result<T> = std::result::Result<T, error::Error>;

use lazy_static::lazy_static;

lazy_static! {
    static ref LOG_QUERIES: bool = std::env::var("LOG_QUERIES")
        .map(|_| true)
        .unwrap_or(false);
}
