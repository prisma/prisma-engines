#![allow(clippy::upper_case_acronyms)]

pub mod dmmf;

mod error;
mod graphql;
mod transactions;

#[cfg(test)]
mod tests;

pub use error::HandlerError;
pub use graphql::*;
pub use transactions::*;

pub type Result<T> = std::result::Result<T, HandlerError>;

#[derive(Debug, serde::Serialize, PartialEq)]
#[serde(untagged)]
pub enum PrismaResponse {
    Single(GQLResponse),
    Multi(GQLBatchResponse),
}

impl PrismaResponse {
    pub fn errors(&self) -> Vec<&GQLError> {
        match self {
            PrismaResponse::Single(ref s) => s.errors().collect(),
            PrismaResponse::Multi(ref m) => m.errors().collect(),
        }
    }
}
