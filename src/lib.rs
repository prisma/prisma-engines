//! # prisma-query
//!
//! prisma-query is an AST and database-specific visitors for creating SQL
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
//! use prisma_query::{ast::*, visitor::{Sqlite, Visitor}};
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
//! use prisma_query::{ast::*, connector::*};
//!
//! fn main() {
//!     let mut conn = Sqlite::new("test.db").unwrap();
//!     let query = Select::default().value(1);
//!     let result = conn.query(query.into()).unwrap();
//!
//!     assert_eq!(
//!         Some(1),
//!         result[0].into_iter().nth(0).and_then(|row| row[0].as_i64()),
//!     );
//! }
//! ```
pub mod ast;
#[cfg(any(
    feature = "mysql-16",
    feature = "postgresql-0_16",
    feature = "rusqlite-0_19"
))]
pub mod connector;
pub mod error;
#[cfg(any(
    feature = "mysql-16",
    feature = "postgresql-0_16",
    feature = "rusqlite-0_19"
))]
pub mod pool;
#[cfg(any(
    feature = "mysql-16",
    feature = "postgresql-0_16",
    feature = "rusqlite-0_19"
))]
pub mod visitor;

pub type Result<T> = std::result::Result<T, error::Error>;
