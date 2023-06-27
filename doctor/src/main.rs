mod kb;
mod pg;
mod query;

use actix_web::{get, http::header::ContentType, post, web, App, HttpResponse, HttpServer, Responder};
use once_cell::sync::Lazy;

use kb::KnowledgeBase;
use query::SubmittedQueryInfo;

static KB: Lazy<KnowledgeBase> = Lazy::new(|| KnowledgeBase::default());

#[get("/index-health")]
async fn index_health() -> impl Responder {
    HttpResponse::NotFound()
}

#[derive(serde::Deserialize)]
struct SlowQueriesArgs {
    threshold: Option<f64>,
    k: Option<i32>,
}

#[get("/slow-queries")]
async fn slow_queries(stats: web::Data<pg::Stats>, args: web::Query<SlowQueriesArgs>) -> impl Responder {
    let threshold = args.threshold.unwrap_or(100.0);
    let k = args.k.unwrap_or(10);

    match stats.slow_queries(threshold, k).await {
        Ok(queries) => HttpResponse::Ok()
            .content_type(ContentType::json())
            .body(serde_json::to_string(&queries).unwrap()),
        Err(e) => HttpResponse::InternalServerError().body(e.to_string()),
    }
}

#[post("/submit-query")]
async fn submit_query(info: web::Json<SubmittedQueryInfo>) -> impl Responder {
    match KB.index(&info) {
        Ok(_) => HttpResponse::Ok().finish(),
        Err(msg) => HttpResponse::InternalServerError().body(msg),
    }
}

#[post("/clear-stats")]
async fn clear_stats(_: web::Data<pg::Stats>) -> impl Responder {
    HttpResponse::NotFound()
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let conn = pg::Stats::init(&database_url, KB.clone());

    let result = HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(conn.clone()))
            .service(slow_queries)
            .service(submit_query)
            .service(clear_stats)
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await;

    println!("🧑‍🔬 Stopping");

    result
}
