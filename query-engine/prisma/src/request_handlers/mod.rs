pub mod graphql;

pub use query_core::schema::QuerySchemaRenderer;
pub use graphql::{GraphQlBody, GraphQlRequestHandler};

use crate::context::PrismaContext;
use serde_json;
use std::{collections::HashMap, fmt::Debug};
use async_trait::async_trait;

#[async_trait]
pub trait RequestHandler {
    type Body: Debug;

    async fn handle<S>(&self, req: S, ctx: &PrismaContext) -> serde_json::Value
    where
        S: Into<PrismaRequest<Self::Body>> + Send + Sync + 'static;
}

pub struct PrismaRequest<T> {
    pub body: T,
    pub headers: HashMap<String, String>,
    pub path: String,
}
