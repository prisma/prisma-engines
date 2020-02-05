pub mod graphql;

pub use graphql::*;
pub use query_core::{response_ir, schema::QuerySchemaRenderer};

use crate::context::PrismaContext;
use async_trait::async_trait;
use std::{collections::HashMap, fmt::Debug, sync::Arc};

#[derive(Debug, serde::Serialize)]
#[serde(untagged)]
pub enum PrismaResponse {
    Single(response_ir::Responses),
    Multi(Vec<PrismaResponse>),
}

#[async_trait]
pub trait RequestHandler {
    type Body: Debug;

    async fn handle<S>(&self, req: S, ctx: &Arc<PrismaContext>) -> PrismaResponse
    where
        S: Into<PrismaRequest<Self::Body>> + Send + Sync + 'static;
}

pub struct PrismaRequest<T> {
    pub body: T,
    pub headers: HashMap<String, String>,
    pub path: String,
}
