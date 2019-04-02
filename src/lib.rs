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
//! ## Database priorities
//!
//! - SQLite will be the first visitor
//! - PostgreSQL
//! - MySQL
//!
//! More databases will be decided later.
//!
//! ## Examples
//!
//! ### Chaining conditions
//!
//! ```
//! use prisma_query::{ast::*, visitor::*};
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
//!         "SELECT `naukio`.* FROM `naukio` WHERE ((`word` = ? AND `age` < ?) AND `paw` = ?) LIMIT -1",
//!         sql_str,
//!     );
//!
//!     assert_eq!(
//!         vec![
//!             ParameterizedValue::Text(String::from("meow")),
//!             ParameterizedValue::Integer(10),
//!             ParameterizedValue::Text(String::from("warm")),
//!         ],
//!         params
//!     );
//! }
//! ```
//!
//! We can chain the conditions together by calling the corresponding conjuctive
//! with the compares. The parameters returned implement the corresponding output
//! trait from the database adapter for easy passing into a prepared statement,
//! in this case [rusqlite](https://github.com/jgallagher/rusqlite).
//!
//! ### Building the conditions as a tree
//!
//! ```
//! use prisma_query::{ast::*, visitor::*};
//!
//! fn main() {
//!     let conditions = ConditionTree::and(
//!         ConditionTree::or("word".equals("meow"), "age".less_than(10)),
//!         "paw".equals("warm"),
//!     );
//!
//!     let query = Select::from_table("naukio").so_that(conditions);
//!     let (sql, params) = Sqlite::build(query);
//!
//!     assert_eq!(
//!         "SELECT `naukio`.* FROM `naukio` WHERE ((`word` = ? OR `age` < ?) AND `paw` = ?) LIMIT -1",
//!         sql,
//!     );
//!
//!     assert_eq!(
//!         vec![
//!             ParameterizedValue::Text(String::from("meow")),
//!             ParameterizedValue::Integer(10),
//!             ParameterizedValue::Text(String::from("warm")),
//!         ],
//!         params
//!     );
//! }
//! ```
//!
//! In cases where more feasible we want to build a `ConditionTree` manually
//! from the input, e.g. when mapping data using an `Into<ConditionTree>` trait.

pub mod ast;
pub mod visitor;
