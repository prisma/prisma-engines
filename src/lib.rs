//! # prisma-query
//!
//! prisma-query is an AST and database-specific visitors for creating SQL
//! statements.
//!
//! Under construction and will go through several rounds of changes. Not meant
//! for production use in the current form.
//!
//! ### Goals:
//!
//! - Query generation when the database and conditions are not known beforehand.
//! - Parameterized queries when possible.
//! - A modular design, separate AST and visitor when extending to new databases.
//! - Database support behind a feature flag.
//!
//! ### Non-goals:
//!
//! - Database-level type-safety in query building or being an ORM.
//!
//! ## Database priorities:
//!
//! - SQLite will be the first Visitor
//! - PostgreSQL
//! - MySQL
//!
//! More databases will be decided later.
//!
//! ## Examples
//!
//! In the following examples we use the feature flag `rusqlite` to get Sqlite support.
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
//!     let query = Select::from("naukio").so_that(conditions);
//!     let (sql, params) = Sqlite::build(query);
//!
//!     assert_eq!(
//!         "SELECT * FROM `naukio` WHERE (`word` = ? AND (`age` < ? AND `paw` = ?)) LIMIT -1",
//!         sql,
//!     );
//!
//!     assert_eq!(
//!         vec![
//!             ParameterizedValue::Text(String::from("meow")),
//!             ParameterizedValue::Integer(10),
//!             ParameterizedValue::Text(String::from("warm")),
//!         ]
//!         params,
//!     );
//! }
//! ```
//!
//! We can chain the conditions together by calling the corresponding conjuctive
//! with the operations. The parameters returned implement the corresponding output
//! trait from the database adapter for easy passing into a prepared statement,
//! in this case [rusqlite](https://github.com/jgallagher/rusqlite).
//!
//! ### Building the conditions as a tree
//!
//! ```
//! use prisma_query::{ast::*, visitor::*};
//!
//! fn main() {
//!     let conditions = ConditionTree::not(ConditionTree::and(
//!         ConditionTree::or("word".equals("meow"), "age".less_than(10)),
//!         "paw".equals("warm"),
//!     ));
//!
//!     let query = Select::from("naukio").so_that(conditions);
//!     let (sql, params) = Sqlite::build(query);
//!
//!     assert_eq!(
//!         "SELECT * FROM `naukio` WHERE (`word` = ? AND (`age` < ? AND `paw` = ?)) LIMIT -1",
//!         sql,
//!     );
//!
//!     assert_eq!(
//!         vec![
//!             ParameterizedValue::Text(String::from("meow")),
//!             ParameterizedValue::Integer(10),
//!             ParameterizedValue::Text(String::from("warm")),
//!         ]
//!         params,
//!     );
//! }
//! ```
//!
//! In some cases we want to build a `ConditionTree` manually from the input,
//! e.g. with an `Into<ConditionTree>` trait. In these cases it is easier to
//! build the conditions as a tree.
pub mod ast;
pub mod visitor;
