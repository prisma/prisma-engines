use std::sync::Arc;

use actix_web::{get, http::header::ContentType, post, web, App, HttpResponse, HttpServer, Responder};
use serde::{Deserialize, Serialize};

#[get("/index-health")]
async fn index_health() -> impl Responder {
    HttpResponse::NotFound()
}

#[get("/slow-queries")]
async fn slow_queries() -> impl Responder {
    const MOCK: &'static str = r#"
    [
        {
            "sql": "SELECT * FROM users WHERE age > 18",
            "prisma_queries": ["model.users.findMany({ where: { age: { gt: 18 } } })"],
            "mean_exec_time": 1.5,
            "num_executions": 1,
            "query_plan": "Seq Scan on public.users  (cost=0.00..8.27 rows=2 width=80)",
            "additional_info": {
            "key": "AdditionalInfo 1"
            }
        },
        {
            "sql": "SELECT COUNT(*) FROM orders WHERE status = 'completed'",
            "prisma_queries": ["model.orders.count({ where: { status: 'completed' } })"],
            "mean_exec_time": 3.0,
            "num_executions": 2,
            "query_plan": "Aggregate  (cost=8.27..8.28 rows=1 width=8)",
            "additional_info": {
            "key": "AdditionalInfo 2"
            }
        },
        {
            "sql": "SELECT * FROM products WHERE price > 100",
            "prisma_queries": ["model.products.findMany({ where: { price: { gt: 100 } } })"],
            "mean_exec_time": 4.5,
            "num_executions": 3,
            "query_plan": "Seq Scan on public.products  (cost=0.00..8.27 rows=4 width=100)",
            "additional_info": {
            "key": "AdditionalInfo 3"
            }
        },
        {
            "sql": "SELECT name, email FROM customers WHERE created_at > '2022-01-01'",
            "prisma_queries": ["model.customers.findMany({ select: { name: true, email: true }, where: { created_at: { gt: '2022-01-01' } } })"],
            "mean_exec_time": 6.0,
            "num_executions": 4,
            "query_plan": "Seq Scan on public.customers  (cost=0.00..8.27 rows=3 width=64)",
            "additional_info": {
            "key": "AdditionalInfo 4"
            }
        },
        {
            "sql": "SELECT * FROM posts WHERE published = true ORDER BY created_at DESC LIMIT 10",
            "prisma_queries": ["model.posts.findMany({ where: { published: true }, orderBy: { created_at: 'desc' }, take: 10 })"],
            "mean_exec_time": 7.5,
            "num_executions": 5,
            "query_plan": "Limit  (cost=8.27..8.32 rows=10 width=76)",
            "additional_info": {
            "key": "AdditionalInfo 5"
            }
        },
        {
            "sql": "SELECT AVG(rating) FROM reviews WHERE product_id = 123",
            "prisma_queries": ["model.reviews.aggregate({ avg: { rating: true }, where: { product_id: 123 } })"],
            "mean_exec_time": 9.0,
            "num_executions": 6,
            "query_plan": "Aggregate  (cost=8.27..8.27 rows=1 width=32)",
            "additional_info": {
            "key": "AdditionalInfo 6"
            }
        }
        ]"#;
    HttpResponse::Ok().content_type(ContentType::json()).body(MOCK)
}

#[post("/submit-query")]
async fn submit_query(q: web::Json<SubmittedQueryInfo>) -> impl Responder {
    HttpResponse::NotFound()
}

#[post("/clear-stats")]
async fn clear_stats(q: web::Json<SubmittedQueryInfo>) -> impl Responder {
    HttpResponse::NotFound()
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| App::new().service(slow_queries).service(submit_query))
        .bind(("127.0.0.1", 8080))?
        .run()
        .await
}

#[derive(Deserialize, Serialize)]
struct SubmittedQueryInfo {
    sql: String,
    tag: String,
    prisma_query: String,
}

type RawQuery = String;
type PrismaQuery = String;
type QueryPlan = String;

#[derive(Serialize)]
struct SlowQuery {
    sql: RawQuery,
    prisma_queries: Vec<PrismaQuery>,
    mean_exec_time: f64,
    num_executions: u64,
    query_plan: QueryPlan,
    additional_info: serde_json::Value,
}
