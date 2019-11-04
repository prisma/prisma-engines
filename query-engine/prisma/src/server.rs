use super::dmmf;
use crate::{
    context::PrismaContext,
    request_handlers::{
        graphql::{GraphQLSchemaRenderer, GraphQlBody, GraphQlRequestHandler},
        PrismaRequest, RequestHandler,
    },
    PrismaResult,
};
use query_core::schema::QuerySchemaRenderer;
use futures::stream::TryStreamExt;
use hyper::header;
use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Error, Method, Request, Response, Server, StatusCode};
use serde_json::json;
use std::{sync::Arc, time::Instant};

#[derive(RustEmbed)]
#[folder = "query-engine/prisma/static_files"]
struct StaticFiles;

pub(crate) struct RequestContext {
    context: PrismaContext,
    graphql_request_handler: GraphQlRequestHandler,
}

pub struct HttpServer;

impl HttpServer {
    pub async fn run(address: ([u8; 4], u16), legacy_mode: bool) -> PrismaResult<()> {
        let now = Instant::now();

        let ctx = Arc::new(RequestContext {
            context: PrismaContext::new(legacy_mode)?,
            graphql_request_handler: GraphQlRequestHandler,
        });

        let service = make_service_fn(|_| {
            let ctx = ctx.clone();

            async { Ok::<_, Error>(service_fn(move |req| Self::routes(ctx.clone(), req))) }
        });

        let address = address.into();
        let server = Server::bind(&address).serve(service);

        trace!("Initialized in {}ms", now.elapsed().as_millis());
        info!("Started http server on {}:{}", address.ip(), address.port());

        server.await.unwrap();

        Ok(())
    }

    async fn routes(ctx: Arc<RequestContext>, req: Request<Body>) -> std::result::Result<Response<Body>, Error> {
        let res = match (req.method(), req.uri().path()) {
            (&Method::POST, "/") => {
                let (parts, chunks) = req.into_parts();
                let body_bytes = chunks.try_concat().await?;

                match serde_json::from_slice(body_bytes.as_ref()) {
                    Ok(body) => {
                        let req = PrismaRequest {
                            body,
                            path: parts.uri.path().into(),
                            headers: parts
                                .headers
                                .iter()
                                .map(|(k, v)| (format!("{}", k), v.to_str().unwrap().into()))
                                .collect(),
                        };

                        Self::http_handler(req, ctx).await
                    }
                    Err(_) => {
                        let mut bad_request = Response::default();
                        *bad_request.status_mut() = StatusCode::BAD_REQUEST;
                        bad_request
                    }
                }
            }

            (&Method::GET, "/") => Self::playground_handler(),
            (&Method::GET, "/status") => Self::status_handler(),

            (&Method::GET, "/sdl") => Self::sdl_handler(ctx),
            (&Method::GET, "/dmmf") => Self::dmmf_handler(ctx),
            (&Method::GET, "/server_info") => Self::server_info_handler(ctx),

            _ => {
                let mut not_found = Response::default();
                *not_found.status_mut() = StatusCode::NOT_FOUND;
                not_found
            }
        };

        Ok(res)
    }

    async fn http_handler(req: PrismaRequest<GraphQlBody>, cx: Arc<RequestContext>) -> Response<Body> {
        let result = cx.graphql_request_handler.handle(req, &cx.context).await;
        let bytes = serde_json::to_vec(&result).unwrap();

        Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(bytes))
            .unwrap()
    }

    fn status_handler() -> Response<Body> {
        let body_data = json!({"status": "ok"});
        let bytes = serde_json::to_vec(&body_data).unwrap();

        Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(bytes))
            .unwrap()
    }

    fn playground_handler() -> Response<Body> {
        let index_html = StaticFiles::get("playground.html").unwrap();

        Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, "text/html")
            .body(Body::from(index_html.into_owned()))
            .unwrap()
    }

    /// Handler for the playground to work with the SDL-rendered query schema.
    /// Serves a raw SDL string created from the query schema.
    fn sdl_handler(cx: Arc<RequestContext>) -> Response<Body> {
        let rendered = GraphQLSchemaRenderer::render(Arc::clone(&cx.context.query_schema()));

        Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, "application/text")
            .body(Body::from(rendered))
            .unwrap()
    }

    /// Renders the Data Model Meta Format.
    /// Only callable if prisma was initialized using a v2 data model.
    fn dmmf_handler(cx: Arc<RequestContext>) -> Response<Body> {
        let dmmf = dmmf::render_dmmf(cx.context.datamodel(), Arc::clone(cx.context.query_schema()));

        let bytes = serde_json::to_vec(&dmmf).unwrap();

        Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(bytes))
            .unwrap()
    }

    /// Simple status endpoint
    fn server_info_handler(cx: Arc<RequestContext>) -> Response<Body> {
        let json = json!({
            "commit": env!("GIT_HASH"),
            "version": env!("CARGO_PKG_VERSION"),
            "primary_connector": cx.context.primary_connector(),
        });

        let bytes = serde_json::to_vec(&json).unwrap();

        Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(bytes))
            .unwrap()
    }
}
