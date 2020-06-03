#![deny(missing_docs)]

use super::dmmf;
use crate::{
    context::PrismaContext,
    request_handlers::{
        graphql::{GraphQLSchemaRenderer, GraphQlRequestHandler},
        PrismaRequest, RequestHandler,
    },
    PrismaResult,
};
use elapsed_middleware::ElapsedMiddleware;

use datamodel::{Configuration, Datamodel};
use query_core::schema::QuerySchemaRenderer;
use serde_json::json;
use tide::http::{mime, StatusCode};
use tide::{Body, Request, Response};

use std::net::SocketAddr;
use std::sync::Arc;

mod elapsed_middleware;

//// Shared application state.
pub(crate) struct State {
    cx: Arc<PrismaContext>,
    enable_playground: bool,
}

impl State {
    /// Create a new instance of `State`.
    fn new(cx: PrismaContext, enable_playground: bool) -> Self {
        Self {
            cx: Arc::new(cx),
            enable_playground,
        }
    }
}

impl Clone for State {
    fn clone(&self) -> Self {
        Self {
            cx: self.cx.clone(),
            enable_playground: self.enable_playground,
        }
    }
}

/// A builder for `HttpServer`.
pub struct HttpServerBuilder {
    /// The address we listen on.
    addr: SocketAddr,
    /// The server configuration passed.
    config: Configuration,
    /// The Prisma data model.
    datamodel: Datamodel,
    /// Are we listening in legacy mode?
    legacy_mode: bool,
    /// Do we enable raw queries?
    ///
    /// Note: this has security implications.
    enable_raw_queries: bool,
    /// Do we enable the GraphQL playground?
    ///
    /// Note: this has security implications.
    enable_playground: bool,
}

impl HttpServerBuilder {
    /// Create a new instance of `HttpServerBuilder`.
    fn new(addr: SocketAddr, config: Configuration, datamodel: Datamodel) -> Self {
        Self {
            addr,
            config,
            datamodel,
            legacy_mode: false,
            enable_playground: false,
            enable_raw_queries: false,
        }
    }

    /// Enable "legacy mode" for prisma-engines.
    pub fn legacy(mut self, val: bool) -> Self {
        self.legacy_mode = val;
        self
    }

    /// Enable raw queries for prisma-engines.
    ///
    /// # Security
    ///
    /// Enabling this setting will allow arbtrary queries to be executed against
    /// the server. This has security implications when exposing Prisma on a
    /// public port.
    pub fn enable_raw_queries(mut self, val: bool) -> Self {
        self.enable_raw_queries = val;
        self
    }

    /// Enable the GraphQL playground.
    pub fn enable_playground(mut self, val: bool) -> Self {
        self.enable_playground = val;
        self
    }

    /// Start the server.
    pub async fn build(self) -> PrismaResult<()> {
        let ctx = PrismaContext::builder(self.config, self.datamodel)
            .legacy(self.legacy_mode)
            .enable_raw_queries(self.enable_raw_queries)
            .build()
            .await?;

        HttpServer::run(self.addr, ctx, self.enable_playground).await
    }
}

pub struct HttpServer;

impl HttpServer {
    /// Create a new HTTP server builder.
    pub fn builder(addr: SocketAddr, config: Configuration, datamodel: Datamodel) -> HttpServerBuilder {
        HttpServerBuilder::new(addr, config, datamodel)
    }

    /// Start the HTTP server with the default options enabled.
    async fn run(addr: SocketAddr, cx: PrismaContext, enable_playground: bool) -> PrismaResult<()> {
        let mut app = tide::with_state(State::new(cx, enable_playground));
        app.middleware(ElapsedMiddleware::new());

        app.at("/").post(graphql_handler);
        app.at("/").get(playground_handler);
        app.at("/sdl").get(sdl_handler);
        app.at("/dmmf").get(dmmf_handler);
        app.at("/server_info").get(server_info_handler);
        app.at("/status").get(|_| async move { Ok(json!({"status": "ok"})) });

        info!("Started http server on {}:{}", addr.ip(), addr.port());
        app.listen(addr).await?;
        Ok(())
    }
}

/// The main query handler. This handles incoming GraphQL queries and passes it
/// to the query engine.
async fn graphql_handler(mut req: Request<State>) -> tide::Result {
    let body = req.body_json().await?;
    let path = req.url().path().to_owned();
    let headers = req.iter().map(|(k, v)| (format!("{}", k), format!("{}", v))).collect();
    let cx = req.state().cx.clone();
    let req = PrismaRequest { body, path, headers };
    let result = GraphQlRequestHandler.handle(req, &cx).await;
    let mut res = Response::new(StatusCode::Ok);
    res.set_body(Body::from_json(&result)?);
    Ok(res)
}

/// Expose the GraphQL playground if enabled.
///
/// # Security
///
/// In production exposing the playground is equivalent to exposing the database
/// on a port. This should never be enabled on production servers.
async fn playground_handler(req: Request<State>) -> tide::Result {
    if !req.state().enable_playground {
        return Ok(Response::new(StatusCode::NotFound));
    }

    let mut res = Response::new(StatusCode::Ok);
    res.set_body(include_bytes!("../../static_files/playground.html").to_vec());
    res.set_content_type(mime::HTML);
    Ok(res)
}

/// Handler for the playground to work with the SDL-rendered query schema.
/// Serves a raw SDL string created from the query schema.
async fn sdl_handler(req: Request<State>) -> tide::Result<impl Into<Response>> {
    let schema = Arc::clone(&req.state().cx.query_schema());
    Ok(GraphQLSchemaRenderer::render(schema))
}

/// Renders the Data Model Meta Format.
/// Only callable if prisma was initialized using a v2 data model.
async fn dmmf_handler(req: Request<State>) -> tide::Result {
    let result = dmmf::render_dmmf(req.state().cx.datamodel(), Arc::clone(req.state().cx.query_schema()));
    let mut res = Response::new(StatusCode::Ok);
    res.set_body(Body::from_json(&result)?);
    Ok(res)
}

/// Simple status endpoint
async fn server_info_handler(req: Request<State>) -> tide::Result<impl Into<Response>> {
    Ok(json!({
        "commit": env!("GIT_HASH"),
        "version": env!("CARGO_PKG_VERSION"),
        "primary_connector": req.state().cx.primary_connector(),
    }))
}
