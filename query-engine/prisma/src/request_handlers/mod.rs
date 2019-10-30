pub mod graphql;

pub use core::schema::QuerySchemaRenderer;
pub use graphql::{GraphQlBody, GraphQlRequestHandler};

use crate::context::PrismaContext;
use serde_json;
use std::collections::HashMap;

pub trait RequestHandler {
    type Body;

    fn handle<S: Into<PrismaRequest<Self::Body>>>(&self, req: S, ctx: &PrismaContext) -> serde_json::Value;
}

pub struct PrismaRequest<T> {
    pub body: T,
    pub headers: HashMap<String, String>,
    pub path: String,
}
