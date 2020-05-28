use super::dmmf;
use crate::{
    context::PrismaContext,
    request_handlers::{
        graphql::{GraphQLSchemaRenderer, GraphQlBody, GraphQlRequestHandler},
        PrismaRequest, RequestHandler,
    },
    PrismaResult,
};

use datamodel::{Configuration, Datamodel};
use query_core::schema::QuerySchemaRenderer;
use serde_json::json;
use tide::http::{headers, mime, StatusCode};
use tide::{Body, Request, Response};

use std::net::SocketAddr;
use std::sync::Arc;

//// Shared application state.
pub(crate) struct State {
    cx: Arc<PrismaContext>,
    graphql_request_handler: GraphQlRequestHandler,
    enable_playground: bool,
}

impl Clone for State {
    fn clone(&self) -> Self {
        Self {
            cx: self.cx.clone(),
            graphql_request_handler: GraphQlRequestHandler,
            enable_playground: self.enable_playground,
        }
    }
}

impl State {
    //// Access the shared application state.
    pub(crate) fn context(&self) -> &Arc<PrismaContext> {
        &self.cx
    }
}

/// A builder for `HttpServer`.
pub struct HttpServerBuilder {
    addr: SocketAddr,
    config: Configuration,
    datamodel: Datamodel,
    legacy_mode: bool,
    enable_raw_queries: bool,
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
        // let now = Instant::now();

        let ctx = State {
            cx: Arc::new(cx),
            graphql_request_handler: GraphQlRequestHandler,
            enable_playground,
        };

        let mut app = tide::with_state(ctx);
        app.at("/").post(|req: Request<State>| async move {
            // let body = req.body_json().await.status(StatusCode::BadRequest)?;
            // let path = req.url().path().as_str().to_owned();
            // let headers = req.headers().clone();
            // let res = Self::http_handler(req, ctx).await.status(StatusCode::BadRequest)?;
            // Ok(res)
            // TODO: impl
            Ok(Response::new(StatusCode::Ok))
        });

        app.at("/").get(playground_handler);
        app.at("/status").get(|_| async move {
            let body = json!({"status": "ok"});
            let mut res = Response::new(StatusCode::Ok);
            res.set_body(Body::from_json(&body)?);
            Ok(res)
        });
        app.at("/sdl").get(sdl_handler);
        app.at("/dmmf").get(dmmf_handler);
        app.at("/server_info").get(server_info_handler);

        // TODO(yoshuawuyts): change this to a middleware.
        // let elapsed = Instant::now().duration_since(start).as_micros() as u64;
        // res.headers_mut().insert("x-elapsed", elapsed.into());
        // trace!("Initialized in {}ms", now.elapsed().as_millis());

        info!("Started http server on {}:{}", addr.ip(), addr.port());

        app.listen(addr).await?;
        Ok(())
    }
}

async fn http_handler(req: PrismaRequest<GraphQlBody>, cx: State) -> tide::Result {
    let result = cx.graphql_request_handler.handle(req, cx.context()).await;
    let mut res = Response::new(StatusCode::Ok);
    res.set_body(Body::from_json(&result)?);
    Ok(res)
}

async fn playground_handler(req: Request<State>) -> tide::Result {
    if !req.state().enable_playground {
        return Ok(Response::new(StatusCode::NotFound));
    }

    let mut res = Response::new(StatusCode::Ok);
    res.set_body(include_bytes!("../static_files/playground.html").to_vec());
    res = res.set_header(headers::CONTENT_ENCODING, mime::HTML);
    Ok(res)
}

/// Handler for the playground to work with the SDL-rendered query schema.
/// Serves a raw SDL string created from the query schema.
async fn sdl_handler(req: Request<State>) -> tide::Result {
    let body = GraphQLSchemaRenderer::render(Arc::clone(&req.state().cx.query_schema()));
    let mut res = Response::new(StatusCode::Ok);
    res.set_body(body);
    res = res.set_header(headers::CONTENT_ENCODING, mime::PLAIN);
    Ok(res)
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
async fn server_info_handler(req: Request<State>) -> tide::Result {
    let body = json!({
        "commit": env!("GIT_HASH"),
        "version": env!("CARGO_PKG_VERSION"),
        "primary_connector": req.state().cx.primary_connector(),
    });

    let mut res = Response::new(StatusCode::Ok);
    res.set_body(Body::from_json(&body)?);
    Ok(res)
}
