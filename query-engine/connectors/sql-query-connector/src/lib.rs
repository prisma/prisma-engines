#![allow(
    clippy::wrong_self_convention,
    clippy::upper_case_acronyms,
    clippy::needless_question_mark,
    clippy::branches_sharing_code,
    clippy::mem_replace_with_default,
    clippy::needless_borrow,
    clippy::needless_collect
)]

mod column_metadata;
mod cursor_condition;
mod database;
mod error;
mod filter_conversion;
mod join_utils;
mod nested_aggregations;
mod ordering;
mod query_arguments_ext;
mod query_builder;
mod query_ext;
mod row;
mod sql_info;

use column_metadata::*;
use filter_conversion::*;
use query_ext::QueryExt;
use row::*;

pub use database::*;
pub use error::SqlError;

type Result<T> = std::result::Result<T, error::SqlError>;
