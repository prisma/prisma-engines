use crate::{context::PrismaContext, opt::PrismaOpt, PrismaResult};
use datamodel::common::preview_features::PreviewFeature;
use hyper::service::{make_service_fn, service_fn};
use hyper::{header::CONTENT_TYPE, Body, HeaderMap, Method, Request, Response, Server, StatusCode};
use opentelemetry::{global, propagation::Extractor, Context};
use query_core::{schema::QuerySchemaRenderer, set_parent_context_from_json_str, TxId};
use query_core::{MetricFormat, MetricRegistry};
use request_handlers::{dmmf, GraphQLSchemaRenderer, GraphQlHandler, TxInput};
use serde_json::json;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Instant;
use tracing::{field, Instrument, Span};
use tracing_opentelemetry::OpenTelemetrySpanExt;

const TRANSACTION_ID_HEADER: &str = "X-transaction-id";

//// Shared application state.
pub struct State {
    cx: Arc<PrismaContext>,
    enable_playground: bool,
    enable_debug_mode: bool,
    enable_itx: bool,
    enable_metrics: bool,
}

impl State {
    /// Create a new instance of `State`.
    fn new(
        cx: PrismaContext,
        enable_playground: bool,
        enable_debug_mode: bool,
        enable_itx: bool,
        enable_metrics: bool,
    ) -> Self {
        Self {
            cx: Arc::new(cx),
            enable_playground,
            enable_debug_mode,
            enable_itx,
            enable_metrics,
        }
    }

    pub fn get_metrics(&self) -> MetricRegistry {
        self.cx.metrics.clone()
    }
}

impl Clone for State {
    fn clone(&self) -> Self {
        Self {
            cx: self.cx.clone(),
            enable_playground: self.enable_playground,
            enable_debug_mode: self.enable_debug_mode,
            enable_itx: self.enable_itx,
            enable_metrics: self.enable_metrics,
        }
    }
}

pub async fn setup(opts: &PrismaOpt, metrics: MetricRegistry) -> PrismaResult<State> {
    let config = opts.configuration(false)?.subject;
    config.validate_that_one_datasource_is_provided()?;

    let span = tracing::info_span!("prisma:engine:connect", user_facing = true);
    let _ = set_parent_context_from_json_str(&span, &(opts.tracing_headers));

    let enable_itx = config
        .preview_features()
        .contains(PreviewFeature::InteractiveTransactions);

    let enable_metrics = config.preview_features().contains(PreviewFeature::Metrics);

    let datamodel = opts.datamodel()?;
    let cx = PrismaContext::builder(config, datamodel)
        .set_metrics(metrics)
        .enable_raw_queries(opts.enable_raw_queries)
        .build()
        .instrument(span)
        .await?;

    let state = State::new(
        cx,
        opts.enable_playground,
        opts.enable_debug_mode,
        enable_itx,
        enable_metrics,
    );
    Ok(state)
}

/// Starts up the graphql query engine server
pub async fn listen(opts: PrismaOpt, metrics: MetricRegistry) -> PrismaResult<()> {
    let state = setup(&opts, metrics).await?;

    let query_engine = make_service_fn(move |_| {
        let state = state.clone();
        async move { Ok::<_, hyper::Error>(service_fn(move |req| routes(state.clone(), req))) }
    });

    let ip = opts.host.parse().expect("Host was not a valid IP address.");
    let addr = SocketAddr::new(ip, opts.port);

    let server = Server::bind(&addr).tcp_nodelay(true).serve(query_engine);

    info!("Started query engine http server on http://{}", addr);

    if let Err(e) = server.await {
        eprintln!("server error: {}", e);
    }

    Ok(())
}

pub async fn routes(state: State, req: Request<Body>) -> Result<Response<Body>, hyper::Error> {
    let start = Instant::now();

    if req.method() == Method::POST && req.uri().path().contains("transaction") && state.enable_itx {
        return handle_transaction(state, req).await;
    }

    if [Method::POST, Method::GET].contains(req.method())
        && req.uri().path().contains("metrics")
        && state.enable_metrics
    {
        return handle_metrics(state, req).await;
    }

    let mut res = match (req.method(), req.uri().path()) {
        (&Method::POST, "/") => graphql_handler(state, req).await?,
        (&Method::GET, "/") if state.enable_playground => playground_handler(),
        (&Method::GET, "/status") => Response::builder()
            .status(StatusCode::OK)
            .header(CONTENT_TYPE, "application/json")
            .body(Body::from(r#"{"status":"ok"}"#))
            .unwrap(),

        (&Method::GET, "/sdl") => {
            let schema = GraphQLSchemaRenderer::render(state.cx.query_schema().clone());

            Response::builder()
                .status(StatusCode::OK)
                .header(CONTENT_TYPE, "application/text")
                .body(Body::from(schema))
                .unwrap()
        }

        (&Method::GET, "/dmmf") => {
            let schema = dmmf::render_dmmf(state.cx.datamodel(), Arc::clone(state.cx.query_schema()));

            Response::builder()
                .status(StatusCode::OK)
                .header(CONTENT_TYPE, "application/json")
                .body(Body::from(serde_json::to_vec(&schema).unwrap()))
                .unwrap()
        }

        (&Method::GET, "/server_info") => {
            let body = json!({
                "commit": env!("GIT_HASH"),
                "version": env!("CARGO_PKG_VERSION"),
                "primary_connector": state.cx.primary_connector(),
            });

            Response::builder()
                .status(StatusCode::OK)
                .header(CONTENT_TYPE, "application/json")
                .body(Body::from(serde_json::to_vec(&body).unwrap()))
                .unwrap()
        }
        _ => Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(Body::empty())
            .unwrap(),
    };

    let elapsed = Instant::now().duration_since(start).as_micros() as u64;
    res.headers_mut().insert("x-elapsed", elapsed.into());

    Ok(res)
}

/// The main query handler. This handles incoming GraphQL queries and passes it
/// to the query engine.
async fn graphql_handler(state: State, req: Request<Body>) -> Result<Response<Body>, hyper::Error> {
    // Check for debug headers if enabled.
    if state.enable_debug_mode {
        return Ok(handle_debug_headers(&req));
    }

    let tx_id = get_transaction_id_from_header(&req);

    let span = if tx_id.is_none() {
        let cx = get_parent_span_context(&req);
        let span = info_span!("prisma:engine", user_facing = true);
        span.set_parent(cx);
        span
    } else {
        Span::none()
    };

    let trace_id = match req.headers().get("traceparent") {
        Some(traceparent) => {
            let s = traceparent.to_str().unwrap_or_default().to_string();
            Some(s)
        }
        _ => None,
    };

    let work = async move {
        let body_start = req.into_body();
        // block and buffer request until the request has completed
        let full_body = hyper::body::to_bytes(body_start).await?;

        match serde_json::from_slice(full_body.as_ref()) {
            Ok(body) => {
                let handler = GraphQlHandler::new(&*state.cx.executor, state.cx.query_schema());
                let result = handler.handle(body, tx_id, trace_id).instrument(span).await;

                let result_bytes = serde_json::to_vec(&result).unwrap();

                let res = Response::builder()
                    .status(StatusCode::OK)
                    .header(CONTENT_TYPE, "application/json")
                    .body(Body::from(result_bytes))
                    .unwrap();

                Ok(res)
            }
            Err(_e) => {
                let res = Response::builder()
                    .status(StatusCode::BAD_REQUEST)
                    .body(Body::empty())
                    .unwrap();

                Ok(res)
            }
        }
    };

    work.await
}

/// Expose the GraphQL playground if enabled.
///
/// # Security
///
/// In production exposing the playground is equivalent to exposing the database
/// on a port. This should never be enabled on production servers.
fn playground_handler() -> Response<Body> {
    let playground = include_bytes!("../../static_files/playground.html").to_vec();

    Response::builder()
        .status(StatusCode::OK)
        .header(CONTENT_TYPE, "text/html")
        .body(Body::from(playground))
        .unwrap()
}

async fn handle_metrics(state: State, req: Request<Body>) -> Result<Response<Body>, hyper::Error> {
    let format = if let Some(query) = req.uri().query() {
        if query.contains("format=json") {
            MetricFormat::Json
        } else {
            MetricFormat::Prometheus
        }
    } else {
        MetricFormat::Prometheus
    };

    let body_start = req.into_body();
    // block and buffer request until the request has completed
    let full_body = hyper::body::to_bytes(body_start).await?;

    let global_labels: HashMap<String, String> = match serde_json::from_slice(full_body.as_ref()) {
        Ok(map) => map,
        Err(_e) => HashMap::new(),
    };

    if format == MetricFormat::Json {
        let metrics = state.cx.metrics.to_json(global_labels);

        let res = Response::builder()
            .status(StatusCode::OK)
            .header(CONTENT_TYPE, "application/json")
            .body(Body::from(metrics.to_string()))
            .unwrap();

        return Ok(res);
    }

    let metrics = state.cx.metrics.to_prometheus(global_labels);

    let res = Response::builder()
        .status(StatusCode::OK)
        .header(CONTENT_TYPE, "text/plain; version=0.0.4")
        .body(Body::from(metrics))
        .unwrap();

    Ok(res)
}

/// Sadly the routing doesn't make it obvious what the transaction routes are:
/// POST /transaction/start -> start a transaction
/// POST /transaction/{tx_id}/commit -> commit a transaction
/// POST /transaction/{tx_id}/rollback -> rollback a transaction
async fn handle_transaction(state: State, req: Request<Body>) -> Result<Response<Body>, hyper::Error> {
    let path = req.uri().path();

    if path.contains("start") {
        return transaction_start_handler(state, req).await;
    }

    let sections: Vec<&str> = path.split('/').collect();

    if sections.len() < 2 {
        return Ok(Response::builder()
            .status(StatusCode::INTERNAL_SERVER_ERROR)
            .body(Body::from("Request does not contain the transaction id"))
            .unwrap());
    }

    let tx_id: TxId = sections[2].into();

    let succuss_resp = Response::builder()
        .status(StatusCode::OK)
        .header(CONTENT_TYPE, "application/json")
        .body(Body::from(r#"{}"#))
        .unwrap();

    if path.contains("commit") {
        match state.cx.executor.commit_tx(tx_id).await {
            Ok(_) => Ok(succuss_resp),
            Err(err) => Ok(err_to_http_resp(err)),
        }
    } else if path.contains("rollback") {
        match state.cx.executor.rollback_tx(tx_id).await {
            Ok(_) => Ok(succuss_resp),
            Err(err) => Ok(err_to_http_resp(err)),
        }
    } else {
        let res = Response::builder()
            .status(StatusCode::BAD_REQUEST)
            .body(Body::empty())
            .unwrap();
        Ok(res)
    }
}

async fn transaction_start_handler(state: State, req: Request<Body>) -> Result<Response<Body>, hyper::Error> {
    let cx = get_parent_span_context(&req);

    let body_start = req.into_body();
    // block and buffer request until the request has completed
    let full_body = hyper::body::to_bytes(body_start).await?;
    let input: TxInput = serde_json::from_slice(full_body.as_ref()).unwrap();

    let span = tracing::info_span!("prisma:engine:itx_runner", user_facing = true, itx_id = field::Empty);
    span.set_parent(cx);

    match state
        .cx
        .executor
        .start_tx(
            state.cx.query_schema().clone(),
            input.max_wait,
            input.timeout,
            input.isolation_level,
        )
        .instrument(span)
        .await
    {
        Ok(tx_id) => {
            let result = json!({ "id": tx_id.to_string() });
            let result_bytes = serde_json::to_vec(&result).unwrap();

            let res = Response::builder()
                .status(StatusCode::OK)
                .header(CONTENT_TYPE, "application/json")
                .body(Body::from(result_bytes))
                .unwrap();

            Ok(res)
        }
        Err(err) => Ok(err_to_http_resp(err)),
    }
}

fn get_transaction_id_from_header(req: &Request<Body>) -> Option<TxId> {
    match req.headers().get(TRANSACTION_ID_HEADER) {
        Some(id_header) => {
            let msg = format!("{} has not been correctly set.", TRANSACTION_ID_HEADER);
            let id = id_header.to_str().unwrap_or(msg.as_str());
            Some(TxId::from(id))
        }

        None => None,
    }
}

/// Handle debug headers inside the main GraphQL endpoint.
fn handle_debug_headers(req: &Request<Body>) -> Response<Body> {
    /// Debug header that triggers a panic in the request thread.
    static DEBUG_NON_FATAL_HEADER: &str = "x-debug-non-fatal";

    /// Debug header that causes the query engine to crash.
    static DEBUG_FATAL_HEADER: &str = "x-debug-fatal";

    let headers = req.headers();

    if headers.contains_key(DEBUG_FATAL_HEADER) {
        info!("Query engine debug fatal error, shutting down.");
        std::process::exit(1)
    } else if headers.contains_key(DEBUG_NON_FATAL_HEADER) {
        let err = user_facing_errors::Error::from_panic_payload(Box::new("Debug panic"));
        let body = Body::from(serde_json::to_vec(&err).unwrap());

        Response::builder().status(StatusCode::OK).body(body).unwrap()
    } else {
        Response::builder().status(StatusCode::OK).body(Body::empty()).unwrap()
    }
}

struct HeaderExtractor<'a>(&'a HeaderMap);

impl<'a> Extractor for HeaderExtractor<'a> {
    fn get(&self, key: &str) -> Option<&str> {
        self.0.get(key).and_then(|hv| hv.to_str().ok())
    }

    fn keys(&self) -> Vec<&str> {
        self.0.keys().map(|hk| hk.as_str()).collect()
    }
}

/// If the client sends us a trace and span id, extracting a new context if the
/// headers are set. If not, returns current context.
fn get_parent_span_context(req: &Request<Body>) -> Context {
    let extractor = HeaderExtractor(req.headers());
    global::get_text_map_propagator(|propagator| propagator.extract(&extractor))
}

fn err_to_http_resp(err: query_core::CoreError) -> Response<Body> {
    let status = match err {
        query_core::CoreError::TransactionError(ref err) => match err {
            query_core::TransactionError::AcquisitionTimeout => 504,
            query_core::TransactionError::AlreadyStarted => todo!(),
            query_core::TransactionError::NotFound => 404,
            query_core::TransactionError::Closed { reason: _ } => 422,
            query_core::TransactionError::Unknown { reason: _ } => 500,
        },

        // All other errors are treated as 500s, most of these paths should never be hit, only connector errors may occur.
        _ => 500,
    };

    let user_error: user_facing_errors::Error = err.into();
    let body = Body::from(serde_json::to_vec(&user_error).unwrap());

    Response::builder().status(status).body(body).unwrap()
}
