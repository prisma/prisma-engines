pub mod graphql;

pub use graphql::*;
pub use query_core::{response_ir, schema::QuerySchemaRenderer};

use std::fmt::Debug;

#[derive(Debug, serde::Serialize, PartialEq)]
#[serde(untagged)]
pub enum PrismaResponse {
    Single(response_ir::Responses),
    Multi(Vec<PrismaResponse>),
}
