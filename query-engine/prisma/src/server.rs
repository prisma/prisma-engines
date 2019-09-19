use super::dmmf;
use crate::{
    context::PrismaContext,
    request_handlers::{
        graphql::{GraphQLSchemaRenderer, GraphQlBody, GraphQlRequestHandler},
        PrismaRequest, RequestHandler,
    },
    PrismaResult,
};
use actix_web::{http::Method, App, HttpRequest, HttpResponse, Json, Responder};
use core::schema::QuerySchemaRenderer;
use std::{sync::Arc, time::Instant};

#[derive(RustEmbed)]
#[folder = "query-engine/prisma/static_files"]
struct StaticFiles;

#[derive(DebugStub)]
pub(crate) struct RequestContext {
    context: PrismaContext,
    #[debug_stub = "#GraphQlRequestHandler#"]
    graphql_request_handler: GraphQlRequestHandler,
}

pub struct HttpServer;

impl HttpServer {
    pub fn run(address: (&'static str, u16), legacy_mode: bool) -> PrismaResult<()>
    {
        let now = Instant::now();

        let sys = actix::System::new("prisma");
        let context = PrismaContext::new(legacy_mode)?;

        let request_context = Arc::new(RequestContext {
            context: context,
            graphql_request_handler: GraphQlRequestHandler,
        });

        let server = actix_web::server::new(move || {
            App::with_state(Arc::clone(&request_context))
                .resource("/", |r| {
                    r.method(Method::POST).with(Self::http_handler);
                    r.method(Method::GET).with(Self::playground_handler);
                })
                .resource("/sdl", |r| r.method(Method::GET).with(Self::sdl_handler))
                .resource("/dmmf", |r| r.method(Method::GET).with(Self::dmmf_handler))
                .resource("/status", |r| r.method(Method::GET).with(Self::status_handler))
        });

        server.bind(address)?.start();

        trace!("Initialized in {}ms", now.elapsed().as_millis());
        info!("Started http server on {}:{}", address.0, address.1);

        sys.run();

        Ok(())
    }

    /// Main handler for query engine requests.
    fn http_handler((json, req): (Json<Option<GraphQlBody>>, HttpRequest<Arc<RequestContext>>)) -> impl Responder {
        let request_context = req.state();
        let req: PrismaRequest<GraphQlBody> = PrismaRequest {
            body: json.clone().unwrap(),
            path: req.path().into(),
            headers: req
                .headers()
                .iter()
                .map(|(k, v)| (format!("{}", k), v.to_str().unwrap().into()))
                .collect(),
        };

        let result = request_context
            .graphql_request_handler
            .handle(req, &request_context.context);

        // TODO this copies the data for some reason.
        serde_json::to_string(&result)
    }

    /// Serves playground html.
    fn playground_handler<T>(_: HttpRequest<T>) -> impl Responder {
        let index_html = StaticFiles::get("playground.html").unwrap();
        HttpResponse::Ok().content_type("text/html").body(index_html)
    }

    /// Handler for the playground to work with the SDL-rendered query schema.
    /// Serves a raw SDL string created from the query schema.
    fn sdl_handler(req: HttpRequest<Arc<RequestContext>>) -> impl Responder {
        let request_context = req.state();

        let rendered = GraphQLSchemaRenderer::render(Arc::clone(&request_context.context.query_schema));
        HttpResponse::Ok().content_type("application/text").body(rendered)
    }

    /// Renders the Data Model Meta Format.
    /// Only callable if prisma was initialized using a v2 data model.
    fn dmmf_handler(req: HttpRequest<Arc<RequestContext>>) -> impl Responder {
        let request_context = req.state();
        let dmmf = dmmf::render_dmmf(
            &request_context.context.dm,
            Arc::clone(&request_context.context.query_schema),
        );
        let serialized = serde_json::to_string(&dmmf).unwrap();

        HttpResponse::Ok().content_type("application/json").body(serialized)
    }

    /// Simple status endpoint
    fn status_handler<T>(_: HttpRequest<T>) -> impl Responder {
        HttpResponse::Ok()
            .content_type("application/json")
            .body("{\"status\": \"ok\"}")
    }
}
