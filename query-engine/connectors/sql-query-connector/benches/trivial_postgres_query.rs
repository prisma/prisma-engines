use std::time::Duration;

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use postgres::NoTls;
use quaint::{pooled::Quaint, prelude::Queryable};
use tokio::runtime::Runtime;
use tokio_postgres::Client as AsyncClient;

fn trivial_query_benchmark(c: &mut Criterion) {
    let url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set to a postgres URL");
    let q = "";

    let mut async_client: Option<AsyncClient> = None;
    let async_runtime = Runtime::new().unwrap();

    async_runtime.block_on(async {
        async_client = Some(pg_async_client(&url).await);
    });
    let async_client = async_client.unwrap();

    let quaint = get_quaint(&url);
    c.bench_function(format!("Quaint conn: {}", q).as_str(), |b| {
        b.iter(|| async_runtime.block_on(async { black_box(bm_quaint(&quaint, q).await) }))
    });

    c.bench_function(format!("pg/tokio raw client: {}", q).as_str(), |b| {
        b.iter(|| {
            async_runtime.block_on(async { black_box(bm_pg_tokio_no_tls_prealloc_conn(&async_client, q).await) })
        });
    });
    async_runtime.block_on(async { drop(async_client) });
}

async fn bm_quaint(quaint: &Quaint, query: &str) {
    let conn = quaint.check_out().await.unwrap();
    conn.execute_raw(&query, &[]).await.unwrap();
}

async fn pg_async_client(postgres_url: &str) -> AsyncClient {
    let (client, connection) = tokio_postgres::connect(postgres_url, NoTls).await.unwrap();
    tokio::spawn(async move { connection.await.unwrap() });
    client
}

async fn bm_pg_tokio_no_tls_prealloc_conn(client: &AsyncClient, query: &str) {
    client.query(query, &[]).await.unwrap();
}

fn get_quaint(postgres_url: &str) -> Quaint {
    let mut builder = Quaint::builder(postgres_url).expect("should connect");

    builder.health_check_interval(std::time::Duration::from_secs(15));
    builder.test_on_check_out(true);
    builder.connection_limit(10);

    builder.pool_timeout(Duration::from_secs(30));

    builder.build()
}

criterion_group!(benches, trivial_query_benchmark);
criterion_main!(benches);
