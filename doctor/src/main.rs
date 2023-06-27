mod kb;
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

#[get("/slow-queries")]
async fn slow_queries() -> impl Responder {
    const MOCK: &'static str = r#"
    [
        {
            "sql": "SELECT * FROM users WHERE age > 18",
            "prisma_queries": ["prisma.users.findMany({ where: { age: { gt: 18 } } })"],
            "mean_exec_time": 1.5,
            "num_executions": 1,
            "query_plan": "Seq Scan on public.users  (cost=0.00..8.27 rows=2 width=80)",
            "additional_info": {
            "key": "AdditionalInfo 1"
            }
        },
        {
            "sql": "SELECT COUNT(*) FROM orders WHERE status = 'completed'",
            "prisma_queries": ["prisma.orders.count({ where: { status: 'completed' } })"],
            "mean_exec_time": 3.0,
            "num_executions": 2,
            "query_plan": "Aggregate  (cost=8.27..8.28 rows=1 width=8)",
            "additional_info": {
            "key": "AdditionalInfo 2"
            }
        },
        {
            "sql": "SELECT * FROM products WHERE price > 100",
            "prisma_queries": ["prisma.products.findMany({ where: { price: { gt: 100 } } })"],
            "mean_exec_time": 4.5,
            "num_executions": 3,
            "query_plan": "Seq Scan on public.products  (cost=0.00..8.27 rows=4 width=100)",
            "additional_info": {
            "key": "AdditionalInfo 3"
            }
        },
        {
            "sql": "SELECT name, email FROM customers WHERE created_at > '2022-01-01'",
            "prisma_queries": ["prisma.customers.findMany({ select: { name: true, email: true }, where: { created_at: { gt: '2022-01-01' } } })"],
            "mean_exec_time": 6.0,
            "num_executions": 4,
            "query_plan": "Seq Scan on public.customers  (cost=0.00..8.27 rows=3 width=64)",
            "additional_info": {
            "key": "AdditionalInfo 4"
            }
        },
        {
            "sql": "SELECT * FROM posts WHERE published = true ORDER BY created_at DESC LIMIT 10",
            "prisma_queries": ["prisma.posts.findMany({ where: { published: true }, orderBy: { created_at: 'desc' }, take: 10 })"],
            "mean_exec_time": 7.5,
            "num_executions": 5,
            "query_plan": "Limit  (cost=8.27..8.32 rows=10 width=76)",
            "additional_info": {
            "key": "AdditionalInfo 5"
            }
        },
        {
            "sql": "SELECT AVG(rating) FROM reviews WHERE product_id = 123",
            "prisma_queries": ["prisma.reviews.aggregate({ avg: { rating: true }, where: { product_id: 123 } })"],
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
async fn submit_query(info: web::Json<SubmittedQueryInfo>) -> impl Responder {
    let query_info = info.0;
    if let Err(msg) = KB.index(&query_info) {
        return HttpResponse::InternalServerError().body(msg);
    }
    HttpResponse::Ok().finish()
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
