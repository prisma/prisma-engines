use crate::context::PrismaContext;
use crate::features::Feature;
use crate::{opt::PrismaOpt, PrismaResult};
use hyper::service::{make_service_fn, service_fn};
use hyper::{header::CONTENT_TYPE, Body, HeaderMap, Method, Request, Response, Server, StatusCode};
use opentelemetry::trace::TraceContextExt;
use opentelemetry::{global, propagation::Extractor};
use query_core::helpers::*;
use query_core::telemetry::capturing::TxTraceExt;
use query_core::{telemetry, ExtendedTransactionUserFacingError, TransactionOptions, TxId};
use request_handlers::{dmmf, render_graphql_schema, RequestBody, RequestHandler};
use serde::Serialize;
use serde_json::json;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Instant;
use tracing::{field, Instrument, Span};
use tracing_opentelemetry::OpenTelemetrySpanExt;

/// Starts up the graphql query engine server
pub async fn listen(cx: Arc<PrismaContext>, opts: &PrismaOpt) -> PrismaResult<()> {
    let query_engine = make_service_fn(move |_| {
        let cx = cx.clone();
        async move { Ok::<_, hyper::Error>(service_fn(move |req| routes(cx.clone(), req))) }
    });

    let ip = opts.host.parse().expect("Host was not a valid IP address.");
    let addr = SocketAddr::new(ip, opts.port);

    let server = Server::bind(&addr).tcp_nodelay(true).serve(query_engine);

    // Note: we call `server.local_addr()` instead of reusing original `addr` because it may contain port 0 to request
    // the OS to assign a free port automatically, and we want to print the address which is actually in use.
    info!(
        ip = %server.local_addr().ip(),
        port = %server.local_addr().port(),
        "Started query engine http server on http://{}",
        server.local_addr()
    );

    if let Err(e) = server.await {
        eprintln!("server error: {e}");
    }

    Ok(())
}

pub(crate) async fn routes(cx: Arc<PrismaContext>, req: Request<Body>) -> Result<Response<Body>, hyper::Error> {
    let start = Instant::now();

    if req.method() == Method::POST && req.uri().path().starts_with("/transaction") {
        return transaction_handler(cx, req).await;
    }

    if [Method::POST, Method::GET].contains(req.method())
        && req.uri().path().starts_with("/metrics")
        && cx.enabled_features.contains(Feature::Metrics)
    {
        return metrics_handler(cx, req).await;
    }

    let mut res = match (req.method(), req.uri().path()) {
        (&Method::POST, "/") => request_handler(cx, req).await?,
        (&Method::GET, "/") if cx.enabled_features.contains(Feature::Playground) => playground_handler(),
        (&Method::GET, "/status") => build_json_response(StatusCode::OK, &json!({"status": "ok"})),

        (&Method::GET, "/sdl") => {
            let schema = render_graphql_schema(cx.query_schema());

            Response::builder()
                .status(StatusCode::OK)
                .header(CONTENT_TYPE, "application/text")
                .body(Body::from(schema))
                .unwrap()
        }

        (&Method::GET, "/dmmf") => {
            let schema = dmmf::render_dmmf(cx.query_schema());

            build_json_response(StatusCode::OK, &schema)
        }

        (&Method::GET, "/server_info") => {
            let body = json!({
                "commit": env!("GIT_HASH"),
                "version": env!("CARGO_PKG_VERSION"),
                "primary_connector": cx.primary_connector(),
            });

            build_json_response(StatusCode::OK, &body)
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

/// The main query handler. This handles incoming requests and passes it
/// to the query engine.
async fn request_handler(cx: Arc<PrismaContext>, req: Request<Body>) -> Result<Response<Body>, hyper::Error> {
    // Check for debug headers if enabled.
    if cx.enabled_features.contains(Feature::DebugMode) {
        return Ok(handle_debug_headers(&req));
    }

    let headers = req.headers();
    let capture_settings = capture_settings(headers);

    let tx_id = transaction_id(headers);
    let tracing_cx = get_parent_span_context(headers);

    let span = if tx_id.is_none() {
        let span = info_span!("prisma:engine", user_facing = true);
        span.set_parent(tracing_cx);
        span
    } else {
        Span::none()
    };

    let mut traceparent = traceparent(headers);
    let mut trace_id = get_trace_id_from_traceparent(traceparent.as_deref());

    if traceparent.is_none() {
        // If telemetry needs to be captured, we use the span trace_id to correlate the logs happening
        // during the different operations within a transaction. The trace_id is propagated in the
        // traceparent header, but if it's not present, we need to synthetically create one for the
        // transaction. This is needed, in case the client is interested in capturing logs and not
        // traces, because:
        //  - The client won't send a traceparent header
        //  - A transaction initial span is created here (prisma:engine:itx_runner) and stored in the
        //    ITXServer for that transaction
        //  - When a query comes in, the graphql handler process it, but we need to tell the capturer
        //    to start capturing logs, and for that we need a trace_id. There are two places were we
        //    could get that information from:
        //      - First, it's the traceparent, but the client didn't send it, because they are only
        //      interested in logs.
        //      - Second, it's the root span for the transaction, but it's not in scope but instead
        //      stored in the ITXServer, in a different tokio task.
        //
        // For the above reasons, we need to create a trace_id that we can predict and use accross the
        // different operations happening within a transaction. So we do it by converting the tx_id
        // into a trace_id, leaning on the fact that the tx_id has more entropy, and there's no
        // information loss.
        if capture_settings.logs_enabled() && tx_id.is_some() {
            let tx_id = tx_id.clone().unwrap();
            traceparent = Some(tx_id.as_traceparent());
            trace_id = tx_id.into_trace_id();
        } else {
            // this is the root span, and we are in a single operation.
            traceparent = Some(get_trace_parent_from_span(&span));
            trace_id = get_trace_id_from_span(&span);
        }
    }
    let capture_config = telemetry::capturing::capturer(trace_id, capture_settings);

    if let telemetry::capturing::Capturer::Enabled(capturer) = &capture_config {
        capturer.start_capturing().await;
    }

    let body_start = req.into_body();
    // block and buffer request until the request has completed
    let full_body = hyper::body::to_bytes(body_start).await?;
    let serialized_body = RequestBody::try_from_slice(full_body.as_ref(), cx.engine_protocol());

    let work = async move {
        match serialized_body {
            Ok(body) => {
                let handler = RequestHandler::new(cx.executor(), cx.query_schema(), cx.engine_protocol());
                let mut result = handler.handle(body, tx_id, traceparent).instrument(span).await;

                if let telemetry::capturing::Capturer::Enabled(capturer) = &capture_config {
                    let telemetry = capturer.fetch_captures().await;
                    if let Some(telemetry) = telemetry {
                        result.set_extension("traces".to_owned(), json!(telemetry.traces));
                        result.set_extension("logs".to_owned(), json!(telemetry.logs));
                    }
                }

                let res = build_json_response(StatusCode::OK, &result);

                Ok(res)
            }
            Err(e) => {
                let ufe: user_facing_errors::Error = request_handlers::HandlerError::query_conversion(format!(
                    "Error parsing {:?} query. {}",
                    cx.engine_protocol(),
                    e
                ))
                .into();

                let res = build_json_response(StatusCode::UNPROCESSABLE_ENTITY, &ufe);

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

async fn metrics_handler(cx: Arc<PrismaContext>, req: Request<Body>) -> Result<Response<Body>, hyper::Error> {
    let requested_json = req.uri().query().map(|q| q.contains("format=json")).unwrap_or_default();
    let body_start = req.into_body();
    // block and buffer request until the request has completed
    let full_body = hyper::body::to_bytes(body_start).await?;

    let global_labels: HashMap<String, String> = match serde_json::from_slice(full_body.as_ref()) {
        Ok(map) => map,
        Err(_e) => HashMap::new(),
    };

    let response = if requested_json {
        let metrics = cx.metrics.to_json(global_labels);

        build_json_response(StatusCode::OK, &metrics)
    } else {
        let metrics = cx.metrics.to_prometheus(global_labels);

        Response::builder()
            .status(StatusCode::OK)
            .header(CONTENT_TYPE, "text/plain; version=0.0.4")
            .body(Body::from(metrics))
            .unwrap()
    };

    Ok(response)
}

/// Sadly the routing doesn't make it obvious what the transaction routes are:
/// POST /transaction/start -> start a transaction
/// POST /transaction/{tx_id}/commit -> commit a transaction
/// POST /transaction/{tx_id}/rollback -> rollback a transaction
async fn transaction_handler(cx: Arc<PrismaContext>, req: Request<Body>) -> Result<Response<Body>, hyper::Error> {
    let path = req.uri().path().to_owned();
    let sections: Vec<&str> = path.split('/').collect();

    if sections.len() == 3 && sections[2] == "start" {
        return transaction_start_handler(cx, req).await;
    }

    if sections.len() == 4 && sections[3] == "commit" {
        return transaction_commit_handler(cx, req, sections[2].into()).await;
    }

    if sections.len() == 4 && sections[3] == "rollback" {
        return transaction_rollback_handler(cx, req, sections[2].into()).await;
    }

    let res = Response::builder()
        .status(StatusCode::BAD_REQUEST)
        .body(Body::from(format!("wrong transaction handler path: {}", &path)))
        .unwrap();
    Ok(res)
}

async fn transaction_start_handler(cx: Arc<PrismaContext>, req: Request<Body>) -> Result<Response<Body>, hyper::Error> {
    let headers = req.headers().to_owned();

    let body_start = req.into_body();
    let full_body = hyper::body::to_bytes(body_start).await?;
    let mut tx_opts: TransactionOptions = serde_json::from_slice(full_body.as_ref()).unwrap();
    let tx_id = if tx_opts.new_tx_id.is_none() {
        tx_opts.with_new_transaction_id()
    } else {
        tx_opts.new_tx_id.clone().unwrap()
    };

    // This is the span we use to instrument the execution of a transaction. This span will be open
    // during the tx execution, and held in the ITXServer for that transaction (see ITXServer])
    let span = info_span!("prisma:engine:itx_runner", user_facing = true, itx_id = field::Empty);

    // If telemetry needs to be captured, we use the span trace_id to correlate the logs happening
    // during the different operations within a transaction. The trace_id is propagated in the
    // traceparent header, but if it's not present, we need to synthetically create one for the
    // transaction. This is needed, in case the client is interested in capturing logs and not
    // traces, because:
    //  - The client won't send a traceparent header
    //  - A transaction initial span is created here (prisma:engine:itx_runner) and stored in the
    //    ITXServer for that transaction
    //  - When a query comes in, the graphql handler process it, but we need to tell the capturer
    //    to start capturing logs, and for that we need a trace_id. There are two places were we
    //    could get that information from:
    //      - First, it's the traceparent, but the client didn't send it, because they are only
    //      interested in logs.
    //      - Second, it's the root span for the transaction, but it's not in scope but instead
    //      stored in the ITXServer, in a different tokio task.
    //
    // For the above reasons, we need to create a trace_id that we can predict and use accross the
    // different operations happening within a transaction. So we do it by converting the tx_id
    // into a trace_id, leaning on the fact that the tx_id has more entropy, and there's no
    // information loss.
    let capture_settings = capture_settings(&headers);
    let traceparent = traceparent(&headers);
    if traceparent.is_none() && capture_settings.logs_enabled() {
        span.set_parent(tx_id.into_trace_context())
    } else {
        span.set_parent(get_parent_span_context(&headers))
    }
    let trace_id = span.context().span().span_context().trace_id();
    let capture_config = telemetry::capturing::capturer(trace_id, capture_settings);

    if let telemetry::capturing::Capturer::Enabled(capturer) = &capture_config {
        capturer.start_capturing().await;
    }

    let result = cx
        .executor
        .start_tx(cx.query_schema().clone(), cx.engine_protocol(), tx_opts)
        .instrument(span)
        .await;

    let telemetry = if let telemetry::capturing::Capturer::Enabled(capturer) = &capture_config {
        capturer.fetch_captures().await
    } else {
        None
    };

    match result {
        Ok(tx_id) => {
            let result = if let Some(telemetry) = telemetry {
                json!({ "id": tx_id.to_string(), "extensions": { "logs": json!(telemetry.logs), "traces": json!(telemetry.traces) } })
            } else {
                json!({ "id": tx_id.to_string() })
            };

            let res = build_json_response(StatusCode::OK, &result);

            Ok(res)
        }
        Err(err) => Ok(err_to_http_resp(err, telemetry)),
    }
}

async fn transaction_commit_handler(
    cx: Arc<PrismaContext>,
    req: Request<Body>,
    tx_id: TxId,
) -> Result<Response<Body>, hyper::Error> {
    let capture_config = capture_config(req.headers(), tx_id.clone());

    if let telemetry::capturing::Capturer::Enabled(capturer) = &capture_config {
        capturer.start_capturing().await;
    }

    let result = cx.executor.commit_tx(tx_id).await;

    let telemetry = if let telemetry::capturing::Capturer::Enabled(capturer) = &capture_config {
        capturer.fetch_captures().await
    } else {
        None
    };

    match result {
        Ok(_) => Ok(empty_json_to_http_resp(telemetry)),
        Err(err) => Ok(err_to_http_resp(err, telemetry)),
    }
}

async fn transaction_rollback_handler(
    cx: Arc<PrismaContext>,
    req: Request<Body>,
    tx_id: TxId,
) -> Result<Response<Body>, hyper::Error> {
    let capture_config = capture_config(req.headers(), tx_id.clone());

    if let telemetry::capturing::Capturer::Enabled(capturer) = &capture_config {
        capturer.start_capturing().await;
    }

    let result = cx.executor.rollback_tx(tx_id).await;

    let telemetry = if let telemetry::capturing::Capturer::Enabled(capturer) = &capture_config {
        capturer.fetch_captures().await
    } else {
        None
    };

    match result {
        Ok(_) => Ok(empty_json_to_http_resp(telemetry)),
        Err(err) => Ok(err_to_http_resp(err, telemetry)),
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

        build_json_response(StatusCode::OK, &err)
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

fn empty_json_to_http_resp(captured_telemetry: Option<telemetry::capturing::storage::Storage>) -> Response<Body> {
    let result = if let Some(telemetry) = captured_telemetry {
        json!({ "extensions": { "logs": json!(telemetry.logs), "traces": json!(telemetry.traces) } })
    } else {
        json!({})
    };

    build_json_response(StatusCode::OK, &result)
}

fn err_to_http_resp(
    err: query_core::CoreError,
    captured_telemetry: Option<telemetry::capturing::storage::Storage>,
) -> Response<Body> {
    let status = match err {
        query_core::CoreError::TransactionError(ref err) => match err {
            query_core::TransactionError::AcquisitionTimeout => StatusCode::GATEWAY_TIMEOUT,
            query_core::TransactionError::AlreadyStarted => todo!(),
            query_core::TransactionError::NotFound => StatusCode::NOT_FOUND,
            query_core::TransactionError::Closed { reason: _ } => StatusCode::UNPROCESSABLE_ENTITY,
            query_core::TransactionError::Unknown { reason: _ } => StatusCode::INTERNAL_SERVER_ERROR,
        },

        // All other errors are treated as 500s, most of these paths should never be hit, only connector errors may occur.
        _ => StatusCode::INTERNAL_SERVER_ERROR,
    };

    let mut err: ExtendedTransactionUserFacingError = err.into();
    if let Some(telemetry) = captured_telemetry {
        err.set_extension("traces".to_owned(), json!(telemetry.traces));
        err.set_extension("logs".to_owned(), json!(telemetry.logs));
    }

    build_json_response(status, &err)
}

fn capture_config(headers: &HeaderMap, tx_id: TxId) -> telemetry::capturing::Capturer {
    let capture_settings = capture_settings(headers);
    let mut traceparent = traceparent(headers);

    if traceparent.is_none() && capture_settings.is_enabled() {
        traceparent = Some(tx_id.as_traceparent())
    }

    let trace_id = get_trace_id_from_traceparent(traceparent.as_deref());

    telemetry::capturing::capturer(trace_id, capture_settings)
}

#[allow(clippy::bind_instead_of_map)]
fn capture_settings(headers: &HeaderMap) -> telemetry::capturing::Settings {
    const CAPTURE_TELEMETRY_HEADER: &str = "X-capture-telemetry";
    let s = if let Some(hv) = headers.get(CAPTURE_TELEMETRY_HEADER) {
        hv.to_str().unwrap_or("")
    } else {
        ""
    };

    telemetry::capturing::Settings::from(s)
}

fn traceparent(headers: &HeaderMap) -> Option<String> {
    const TRACEPARENT_HEADER: &str = "traceparent";

    let value = headers
        .get(TRACEPARENT_HEADER)
        .and_then(|h| h.to_str().ok())
        .map(|s| s.to_owned());

    let is_valid_traceparent = |s: &String| s.split_terminator('-').count() >= 4;

    value.filter(is_valid_traceparent)
}

fn transaction_id(headers: &HeaderMap) -> Option<TxId> {
    const TRANSACTION_ID_HEADER: &str = "X-transaction-id";
    headers
        .get(TRANSACTION_ID_HEADER)
        .and_then(|h| h.to_str().ok())
        .map(TxId::from)
}

/// If the client sends us a trace and span id, extracting a new context if the
/// headers are set. If not, returns current context.
fn get_parent_span_context(headers: &HeaderMap) -> opentelemetry::Context {
    let extractor = HeaderExtractor(headers);
    global::get_text_map_propagator(|propagator| propagator.extract(&extractor))
}

fn build_json_response<T>(status_code: StatusCode, value: &T) -> Response<Body>
where
    T: ?Sized + Serialize,
{
    let result_bytes = serde_json::to_vec(value).unwrap();

    Response::builder()
        .status(status_code)
        .header(CONTENT_TYPE, "application/json")
        .header("QE-Content-Length", result_bytes.len()) // this header is read by Accelerate
        .body(Body::from(result_bytes))
        .unwrap()
}
