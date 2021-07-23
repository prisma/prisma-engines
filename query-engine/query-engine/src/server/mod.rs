#![deny(missing_docs)]

use crate::context::PrismaContext;
use crate::opt::PrismaOpt;
use crate::PrismaResult;

use elapsed_middleware::ElapsedMiddleware;
use opentelemetry::{global, Context};
use query_core::schema::QuerySchemaRenderer;
use query_core::TxId;
use request_handlers::{dmmf, GraphQLSchemaRenderer, GraphQlBody, GraphQlHandler};
use serde_json::json;
use tide::http::{mime, StatusCode};
use tide::{prelude::*, Body, Request, Response};
use tide_server_timing::TimingMiddleware;
use tracing::Level;
use tracing_futures::Instrument;
use tracing_opentelemetry::OpenTelemetrySpanExt;

use std::{collections::HashMap, sync::Arc};

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
#[tracing::instrument(skip(opts))]
pub async fn listen(opts: PrismaOpt) -> PrismaResult<()> {
    let config = opts.configuration(false)?.subject;
    config.validate_that_one_datasource_is_provided()?;

    let datamodel = opts.datamodel()?;
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

    // Transaction routes.
    app.at("/transaction/start").post(transaction_start_handler);
    app.at("/transaction/:id/commit").post(transaction_commit_handler);
    app.at("/transaction/:id/rollback").post(transaction_rollback_handler);

    // Start the Tide server and log the server details.
    // NOTE: The `info!` statement is essential for the correct working of the client.
    let mut listener = match opts.unix_path() {
        Some(path) => app.bind(format!("http+unix://{}", path)).await?,
        None => app.bind(format!("{}:{}", opts.host.as_str(), opts.port)).await?,
    };

    info!("Started http server on {}", listener);
    listener.accept().await?;
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

    let cx = get_parent_span_context(&req);
    let span = tracing::span!(Level::TRACE, "graphql_handler");
    span.set_parent(cx);

    let tx_id = req
        .header("X-transaction-id")
        .map(|values| values.last().to_string())
        .map(TxId::from);

    let work = async move {
        let body: GraphQlBody = req.body_json().await?;
        let cx = req.state().cx.clone();

        let handler = GraphQlHandler::new(&*cx.executor, cx.query_schema());
        let result = handler.handle(body, tx_id).await;

        let mut res = Response::new(StatusCode::Ok);
        res.set_body(Body::from_json(&result)?);

        Ok(res)
    };

    work.instrument(span).await
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

#[derive(Debug, Deserialize)]
struct TxInput {
    /// Maximum wait time in seconds.
    pub max_wait: u64,

    /// Time in seconds after which the transaction rolls back automatically.
    pub timeout: u64,
}

async fn transaction_start_handler(mut req: Request<State>) -> tide::Result<impl Into<Response>> {
    let input: TxInput = req.body_json().await?;
    let state = req.state();
    let tx_id = state.cx.executor.start_tx(input.max_wait, input.timeout).await?;

    Ok(json!({
        "id": tx_id.to_string(),
    }))
}

async fn transaction_commit_handler(req: Request<State>) -> tide::Result<impl Into<Response>> {
    let tx_id = TxId::from(req.param("id")?);
    let state = req.state();

    state.cx.executor.commit_tx(tx_id).await?;

    Ok(Response::new(StatusCode::Ok))
}

async fn transaction_rollback_handler(req: Request<State>) -> tide::Result<impl Into<Response>> {
    let tx_id = TxId::from(req.param("id")?);
    let state = req.state();

    state.cx.executor.rollback_tx(tx_id).await?;

    Ok(Response::new(StatusCode::Ok))
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
        let err = user_facing_errors::Error::from_panic_payload(Box::new("Debug panic"));
        let mut res = Response::new(200);
        res.set_body(Body::from_json(&err)?);
        Ok(Some(res))
    } else {
        Ok(None)
    }
}

/// If the client sends us a trace and span id, extracting a new context if the
/// headers are set. If not, returns current context.
fn get_parent_span_context(req: &Request<State>) -> Context {
    let headers: HashMap<String, String> = req
        .iter()
        .filter_map(|(hn, hvs)| hvs.get(0).map(|hv| (hn.as_str().into(), hv.as_str().into())))
        .collect();

    global::get_text_map_propagator(|propagator| propagator.extract(&headers))
}
