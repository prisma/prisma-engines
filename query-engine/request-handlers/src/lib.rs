#![allow(
    clippy::upper_case_acronyms,
    clippy::bool_assert_comparison,
    clippy::mem_replace_with_default,
    clippy::needless_borrow
)]

pub mod dmmf;
mod error;
mod graphql;

#[cfg(test)]
mod tests;

pub use error::HandlerError;
pub use graphql::*;

pub type Result<T> = std::result::Result<T, HandlerError>;

#[derive(Debug, serde::Serialize, PartialEq)]
#[serde(untagged)]
pub enum PrismaResponse {
    Single(GQLResponse),
    Multi(GQLBatchResponse),
}
