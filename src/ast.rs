//! An abstract syntax tree for SQL queries.
//!
//! The ast module handles everything related building abstract SQL queries
//! without going into database-level specifics. Everything related to the
//! actual query building is in the [visitor](../visitor/index.html) module.
//!
//! For prelude, all important imports are in `quaint::ast::*`.
mod column;
mod compare;
mod conditions;
mod conjunctive;
mod delete;
mod expression;
mod function;
mod grouping;
mod insert;
mod join;
mod ops;
mod ordering;
mod over;
mod query;
mod row;
mod select;
mod table;
mod union;
mod update;
mod values;

pub use column::Column;
pub use compare::{Comparable, Compare};
pub use conditions::ConditionTree;
pub use conjunctive::Conjunctive;
pub use delete::Delete;
pub use expression::*;
pub use function::*;
pub use grouping::*;
pub use insert::*;
pub use join::{Join, JoinData, Joinable};
pub use ops::*;
pub use ordering::{IntoOrderDefinition, Order, OrderDefinition, Orderable, Ordering};
pub use over::*;
pub use query::Query;
pub use row::Row;
pub use select::Select;
pub use table::*;
pub use union::Union;
pub use update::*;
pub use values::{Value, Values};

#[cfg(any(feature = "sqlite", feature = "mysql", feature = "postgresql"))]
pub(crate) use values::Params;
