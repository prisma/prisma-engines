#![allow(clippy::upper_case_acronyms)]

pub mod dmmf;

mod error;
mod handler;
mod protocols;
mod response;

pub use error::HandlerError;
pub use handler::*;
pub use protocols::{graphql::*, json::*, RequestBody};
pub use response::*;

pub type Result<T> = std::result::Result<T, HandlerError>;

#[derive(Debug, serde::Serialize, PartialEq)]
#[serde(untagged)]
pub enum PrismaResponse {
    Single(GQLResponse),
    Multi(GQLBatchResponse),
}

impl PrismaResponse {
    pub fn has_errors(&self) -> bool {
        match self {
            PrismaResponse::Single(x) => x.has_errors(),
            PrismaResponse::Multi(x) => x.has_errors(),
        }
    }

    pub fn set_extension(&mut self, key: String, val: serde_json::Value) {
        match self {
            Self::Single(r) => r.set_extension(key, val),
            Self::Multi(r) => r.set_extension(key, val),
        }
    }
}
