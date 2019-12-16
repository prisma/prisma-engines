pub mod graphql;

pub use graphql::{GraphQlBody, GraphQlRequestHandler};
pub use query_core::{schema::QuerySchemaRenderer, response_ir};

use crate::context::PrismaContext;
use async_trait::async_trait;
use std::{collections::HashMap, fmt::Debug};

#[async_trait]
pub trait RequestHandler {
    type Body: Debug;

    async fn handle<S>(&self, req: S, ctx: &PrismaContext) -> response_ir::Responses
    where
        S: Into<PrismaRequest<Self::Body>> + Send + Sync + 'static;
}

pub struct PrismaRequest<T> {
    pub body: T,
    pub headers: HashMap<String, String>,
    pub path: String,
}
