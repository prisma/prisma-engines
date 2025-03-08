use crate::context::PrismaContext;
use crate::features::Feature;
use crate::logger::TracingConfig;
use crate::{opt::PrismaOpt, PrismaResult};
use hyper::service::{make_service_fn, service_fn};
use hyper::{header::CONTENT_TYPE, Body, HeaderMap, Method, Request, Response, Server, StatusCode};
use query_core::{ExtendedUserFacingError, TransactionOptions, TxId};
use request_handlers::{dmmf, render_graphql_schema, RequestBody, RequestHandler};
use serde::Serialize;
use serde_json::json;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};
use telemetry::exporter::{CaptureSettings, CaptureTarget, TraceData};
use telemetry::{NextId, RequestId, TraceParent};
use tracing::{Instrument, Span};

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

        (&Method::GET, "/boot_trace") => {
            let trace = cx.logger.exporter().stop_capturing(cx.boot_request_id).await;
            build_json_response(StatusCode::OK, &trace)
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
    let tx_id = try_get_transaction_id(headers);
    let (span, request_id, traceparent) = setup_telemetry(
        &cx,
        info_span!(
            "prisma:engine:query",
            user_facing = true,
            request_id = tracing::field::Empty,
        ),
        headers,
    )
    .await;

    let query_timeout = query_timeout(headers);

    let buffer = hyper::body::to_bytes(req.into_body()).await?;
    let request_body = RequestBody::try_from_slice(buffer.as_ref(), cx.engine_protocol());

    let work = {
        let cx = Arc::clone(&cx);
        async move {
            match request_body {
                Ok(body) => {
                    let handler = RequestHandler::new(cx.executor(), cx.query_schema(), cx.engine_protocol());
                    let mut result = handler.handle(body, tx_id, traceparent).instrument(span).await;

                    if cx.logger.tracing_config().should_capture() {
                        if let Some(trace) = cx.logger.exporter().stop_capturing(request_id).await {
                            result.set_extension("traces".to_owned(), json!(trace.spans));
                            result.set_extension("logs".to_owned(), json!(trace.events));
                        }
                    }

                    let res = build_json_response(StatusCode::OK, &result);

                    Ok(res)
                }

                Err(e) => {
                    let ufe: user_facing_errors::Error = request_handlers::HandlerError::query_conversion(format!(
                        "Error parsing {:?} query. Ensure that engine protocol of the client and the engine matches. {}",
                        cx.engine_protocol(),
                        e
                    ))
                    .into();

                    let res = build_json_response(StatusCode::UNPROCESSABLE_ENTITY, &ufe);

                    Ok(res)
                }
            }
        }
    };

    let query_timeout_fut = async {
        match query_timeout {
            Some(timeout) => tokio::time::sleep(timeout).await,
            // Never return if timeout isn't set.
            None => std::future::pending().await,
        }
    };

    tokio::select! {
        _ = query_timeout_fut => {
            let captured_telemetry = if cx.logger.tracing_config().should_capture() {
                cx.logger.exporter().stop_capturing(request_id).await
            } else {
                None
            };

            // Note: this relies on the fact that client will rollback the transaction after the
            // error. If the client continues using this transaction (and later commits it), data
            // corruption might happen because some write queries (but not all of them) might be
            // already executed by the database before the timeout is fired.
            Ok(err_to_http_resp(query_core::CoreError::QueryTimeout, captured_telemetry))
        }
        result = work => {
            result
        }
    }
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

    let global_labels: HashMap<String, String> = serde_json::from_slice(full_body.as_ref()).unwrap_or_default();

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

    let tx_opts = match serde_json::from_slice::<TransactionOptions>(full_body.as_ref()) {
        Ok(opts) => {
            if opts.new_tx_id.is_none() {
                opts.with_new_transaction_id()
            } else {
                opts
            }
        }
        Err(_) => {
            return Ok(Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .body(Body::from("Invalid transaction options"))
                .unwrap())
        }
    };

    let (span, request_id, _traceparent) = setup_telemetry(
        &cx,
        info_span!(
            "prisma:engine:start_transaction",
            user_facing = true,
            request_id = tracing::field::Empty,
        ),
        &headers,
    )
    .await;

    let result = cx
        .executor
        .start_tx(cx.query_schema().clone(), cx.engine_protocol(), tx_opts)
        .instrument(span)
        .await;

    let captured_telemetry = if cx.logger.tracing_config().should_capture() {
        cx.logger.exporter().stop_capturing(request_id).await
    } else {
        None
    };

    match result {
        Ok(tx_id) => {
            let result = if let Some(trace) = captured_telemetry {
                json!({ "id": tx_id, "extensions": { "logs": json!(trace.events), "traces": json!(trace.spans) } })
            } else {
                json!({ "id": tx_id })
            };

            let res = build_json_response(StatusCode::OK, &result);

            Ok(res)
        }

        Err(err) => Ok(err_to_http_resp(
            err,
            match cx.logger.tracing_config() {
                TracingConfig::LogsAndTracesInResponse => cx.logger.exporter().stop_capturing(request_id).await,
                _ => None,
            },
        )),
    }
}

async fn transaction_commit_handler(
    cx: Arc<PrismaContext>,
    req: Request<Body>,
    tx_id: TxId,
) -> Result<Response<Body>, hyper::Error> {
    let (span, request_id, _traceparent) = setup_telemetry(
        &cx,
        info_span!(
            "prisma:engine:commit_transaction",
            user_facing = true,
            request_id = tracing::field::Empty,
        ),
        req.headers(),
    )
    .await;

    let result = cx.executor.commit_tx(tx_id).instrument(span).await;

    let telemetry = if cx.logger.tracing_config().should_capture() {
        cx.logger.exporter().stop_capturing(request_id).await
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
    let (span, request_id, _traceparent) = setup_telemetry(
        &cx,
        info_span!(
            "prisma:engine:rollback_transaction",
            user_facing = true,
            request_id = tracing::field::Empty,
        ),
        req.headers(),
    )
    .await;

    let result = cx.executor.rollback_tx(tx_id).instrument(span).await;

    let telemetry = if cx.logger.tracing_config().should_capture() {
        cx.logger.exporter().stop_capturing(request_id).await
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

fn empty_json_to_http_resp(captured_telemetry: Option<TraceData>) -> Response<Body> {
    let result = if let Some(telemetry) = captured_telemetry {
        json!({ "extensions": { "logs": json!(telemetry.events), "traces": json!(telemetry.spans) } })
    } else {
        json!({})
    };

    build_json_response(StatusCode::OK, &result)
}

fn err_to_http_resp(err: query_core::CoreError, captured_telemetry: Option<TraceData>) -> Response<Body> {
    let status = match err {
        query_core::CoreError::TransactionError(ref err) => match err {
            query_core::TransactionError::AcquisitionTimeout => StatusCode::GATEWAY_TIMEOUT,
            query_core::TransactionError::AlreadyStarted => todo!(),
            query_core::TransactionError::NotFound => StatusCode::NOT_FOUND,
            query_core::TransactionError::Closed { reason: _ } => StatusCode::UNPROCESSABLE_ENTITY,
            query_core::TransactionError::Unknown { reason: _ } => StatusCode::INTERNAL_SERVER_ERROR,
        },

        query_core::CoreError::QueryTimeout => StatusCode::REQUEST_TIMEOUT,

        // All other errors are treated as 500s, most of these paths should never be hit, only connector errors may occur.
        _ => StatusCode::INTERNAL_SERVER_ERROR,
    };

    let mut err: ExtendedUserFacingError = err.into();
    if let Some(telemetry) = captured_telemetry {
        err.set_extension("traces".to_owned(), json!(telemetry.spans));
        err.set_extension("logs".to_owned(), json!(telemetry.events));
    }

    build_json_response(status, &err)
}

async fn setup_telemetry(
    cx: &Arc<PrismaContext>,
    span: Span,
    headers: &HeaderMap,
) -> (Span, RequestId, Option<TraceParent>) {
    let request_id = RequestId::next();
    span.record("request_id", request_id.into_u64());

    let capture_settings = match cx.logger.tracing_config() {
        TracingConfig::LogsAndTracesInResponse => headers
            .get("X-capture-telemetry")
            .and_then(|value| value.to_str().ok())
            .unwrap_or_default()
            .parse::<CaptureSettings>()
            .unwrap_or_default(),
        TracingConfig::StdoutLogsAndTracesInResponse => CaptureSettings::new(CaptureTarget::Spans),
        TracingConfig::StdoutLogsOnly => CaptureSettings::none(),
    };

    let traceparent = headers
        .get("traceparent")
        .and_then(|header| header.to_str().ok())
        .and_then(|value| value.parse::<TraceParent>().ok());

    if cx.logger.tracing_config().should_capture() {
        cx.logger.exporter().start_capturing(request_id, capture_settings).await;
    }

    (span, request_id, traceparent)
}

fn try_get_transaction_id(headers: &HeaderMap) -> Option<TxId> {
    headers
        .get("X-transaction-id")
        .and_then(|h| h.to_str().ok())
        .map(TxId::from)
}

fn query_timeout(headers: &HeaderMap) -> Option<Duration> {
    headers
        .get("X-query-timeout")
        .and_then(|h| h.to_str().ok())
        .and_then(|value| value.parse::<u64>().ok())
        .map(Duration::from_millis)
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
