#![deny(missing_docs)]

use crate::context::PrismaContext;
use crate::dmmf;
use crate::opt::PrismaOpt;
use crate::request_handlers::graphql::{self, GraphQLSchemaRenderer, GraphQlBody};
use crate::PrismaResult;
use elapsed_middleware::ElapsedMiddleware;

use query_core::schema::QuerySchemaRenderer;
use serde_json::json;
use tide::http::{mime, StatusCode};
use tide::{Body, Request, Response};
use tide_server_timing::TimingMiddleware;

use std::sync::Arc;

mod elapsed_middleware;

//// Shared application state.
pub(crate) struct State {
    cx: Arc<PrismaContext>,
    enable_playground: bool,
    enable_debug_mode: bool,
}

impl State {
    /// Create a new instance of `State`.
    fn new(cx: PrismaContext, enable_playground: bool, enable_debug_mode: bool) -> Self {
        Self {
            cx: Arc::new(cx),
            enable_playground,
            enable_debug_mode,
        }
    }
}

impl Clone for State {
    fn clone(&self) -> Self {
        Self {
            cx: self.cx.clone(),
            enable_playground: self.enable_playground,
            enable_debug_mode: self.enable_debug_mode,
        }
    }
}

/// Create a new server and listen.
pub async fn listen(opts: PrismaOpt) -> PrismaResult<()> {
    let config = opts
        .configuration(false)?
        .subject
        .validate_that_one_datasource_is_provided()?;
    let datamodel = opts.datamodel(false)?;
    let cx = PrismaContext::builder(config, datamodel)
        .legacy(opts.legacy)
        .enable_raw_queries(opts.enable_raw_queries)
        .build()
        .await?;

    let mut app = tide::with_state(State::new(cx, opts.enable_playground, opts.enable_debug_mode));
    app.with(ElapsedMiddleware::new());

    if opts.enable_playground {
        app.with(TimingMiddleware::new());
    }

    app.at("/").post(graphql_handler);
    app.at("/").get(playground_handler);
    app.at("/sdl").get(sdl_handler);
    app.at("/dmmf").get(dmmf_handler);
    app.at("/server_info").get(server_info_handler);
    app.at("/status").get(|_| async move { Ok(json!({"status": "ok"})) });

    // NOTE: This println is essential for the correct working of the client.
    info!("Started http server");

    // Start the Tide server and log the server details.
    // TODO: Tide should have a panicking listen_unix impl.
    if let Some(path) = opts.unix_path() {
        app.listen(&*format!("http+unix://{}", path)).await?;
    } else {
        app.listen((&*opts.host, opts.port)).await?;
    }
    Ok(())
}

/// The main query handler. This handles incoming GraphQL queries and passes it
/// to the query engine.
async fn graphql_handler(mut req: Request<State>) -> tide::Result {
    // Check for debug headers if enabled.
    if req.state().enable_debug_mode {
        if let Some(res) = handle_debug_headers(&req).await? {
            return Ok(res.into());
        }
    }

    let body: GraphQlBody = req.body_json().await?;
    let cx = req.state().cx.clone();
    let result = graphql::handle(body, cx).await;
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

/// Handle debug headers inside the main GraphQL endpoint.
async fn handle_debug_headers(req: &Request<State>) -> tide::Result<Option<impl Into<Response>>> {
    /// Debug header that triggers a panic in the request thread.
    static DEBUG_NON_FATAL_HEADER: &str = "x-debug-non-fatal";

    /// Debug header that causes the query engine to crash.
    static DEBUG_FATAL_HEADER: &str = "x-debug-fatal";

    if req.header(DEBUG_FATAL_HEADER).is_some() {
        info!("Query engine debug fatal error, shutting down.");
        std::process::exit(1)
    } else if req.header(DEBUG_NON_FATAL_HEADER).is_some() {
        let err = user_facing_errors::Error::from_panic_payload(&String::from("Debug panic"));
        let mut res = Response::new(200);
        res.set_body(Body::from_json(&err)?);
        Ok(Some(res))
    } else {
        Ok(None)
    }
}
