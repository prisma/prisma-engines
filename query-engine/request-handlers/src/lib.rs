#![allow(clippy::upper_case_acronyms)]

pub mod dmmf;

mod error;
mod graphql;

pub use error::HandlerError;
pub use graphql::*;

pub type Result<T> = std::result::Result<T, HandlerError>;

#[derive(Debug, serde::Serialize, PartialEq)]
#[serde(untagged)]
pub enum PrismaResponse {
    Single(GQLResponse),
    Multi(GQLBatchResponse),
}
