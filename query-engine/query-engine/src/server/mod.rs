use hyper::header::CONTENT_TYPE;
use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, HeaderMap, Method, Request, Response, Server, StatusCode};
#[cfg(unix)]
use hyperlocal::UnixServerExt;
use opentelemetry::propagation::Extractor;
use opentelemetry::{global, Context};
use query_core::QuerySchemaRenderer;
use request_handlers::{dmmf, GraphQLSchemaRenderer, GraphQlHandler};
#[cfg(unix)]
use std::{fs, path::Path};
use std::{net::SocketAddr, sync::Arc, time::Instant};
use tracing::Level;
use tracing_futures::Instrument;
use tracing_opentelemetry::OpenTelemetrySpanExt;

use crate::{context::PrismaContext, opt::PrismaOpt, PrismaResult};

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

#[tracing::instrument(skip(opts))]
pub async fn listen(opts: PrismaOpt) -> PrismaResult<()> {
    let datamodel = opts.datamodel()?;

    let config = opts.configuration(false)?.subject;
    config.validate_that_one_datasource_is_provided()?;

    let cx = PrismaContext::builder(config, datamodel)
        .legacy(opts.legacy)
        .enable_raw_queries(opts.enable_raw_queries)
        .build()
        .await?;

    let state = State::new(cx, opts.enable_playground, opts.enable_debug_mode);

    match opts.unix_path() {
        #[cfg(unix)]
        Some(path_str) => {
            let query_engine = make_service_fn(move |_| {
                let state = state.clone();
                async move { Ok::<_, hyper::Error>(service_fn(move |req| routes(state.clone(), req))) }
            });

            let path = Path::new(&path_str);

            if path.exists() {
                fs::remove_file(path).unwrap();
            }

            let server = Server::bind_unix(path).unwrap();
            info!("Started http server on {}", path_str);
            server.serve(query_engine).await.unwrap();
        }
        _ => {
            let query_engine = make_service_fn(move |_| {
                let state = state.clone();
                async move { Ok::<_, hyper::Error>(service_fn(move |req| routes(state.clone(), req))) }
            });

            let ip = opts.host.parse().expect("Host was not a valid IP address.");
            let addr = SocketAddr::new(ip, opts.port);

            let server = Server::bind(&addr);
            info!("Started http server on {}", addr);
            server.serve(query_engine).await.unwrap();
        }
    };

    Ok(())
}

async fn routes(state: State, req: Request<Body>) -> Result<Response<Body>, hyper::Error> {
    let start = Instant::now();

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
            let body = serde_json::json!({
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

async fn graphql_handler(state: State, req: Request<Body>) -> Result<Response<Body>, hyper::Error> {
    // Check for debug headers if enabled.
    if state.enable_debug_mode {
        return Ok(handle_debug_headers(&req));
    }

    let cx = get_parent_span_context(&req);
    let span = tracing::span!(Level::TRACE, "graphql_handler");
    span.set_parent(cx);

    let work = async move {
        let (_, body) = req.into_parts();
        let bytes = hyper::body::to_bytes(body).await?;

        match serde_json::from_slice(bytes.as_ref()) {
            Ok(body) => {
                let handler = GraphQlHandler::new(&*state.cx.executor, state.cx.query_schema());
                let result = handler.handle(body).await;
                let bytes = serde_json::to_vec(&result).unwrap();

                let res = Response::builder()
                    .status(StatusCode::OK)
                    .header(CONTENT_TYPE, "application/json")
                    .body(Body::from(bytes))
                    .unwrap();

                Ok(res)
            }
            Err(_) => {
                let res = Response::builder()
                    .status(StatusCode::BAD_REQUEST)
                    .body(Body::empty())
                    .unwrap();

                Ok(res)
            }
        }
    };

    work.instrument(span).await
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
        .header("content-type", "text/html")
        .body(Body::from(playground))
        .unwrap()
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
