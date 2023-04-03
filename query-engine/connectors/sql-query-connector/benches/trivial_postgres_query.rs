use std::time::Duration;

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use postgres::{Client, NoTls};
use quaint::{
    pooled::{PooledConnection, Quaint},
    prelude::Queryable,
};
use tokio::runtime::Runtime;
use tokio_postgres::Client as AsyncClient;

fn trivial_query_benchmark(c: &mut Criterion) {
    let url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set to a postgres URL");
    let q = "";

    let mut conn: Option<PooledConnection> = None;
    let mut async_client: Option<AsyncClient> = None;
    let async_runtime = Runtime::new().unwrap();

    async_runtime.block_on(async {
        conn = Some(quaint_conn(&url).await);
        async_client = Some(pg_async_client(&url).await);
    });
    let conn = conn.unwrap();
    let async_client = async_client.unwrap();

    c.bench_function(format!("Quaint pre-allocated conn: {}", q).as_str(), |b| {
        b.iter(|| async_runtime.block_on(async { black_box(bm_quaint_prealloc_conn(&conn, q).await) }))
    });
    async_runtime.block_on(async { drop(conn) });

    let quaint = get_quaint(&url);
    c.bench_function(format!("Quaint conn: {}", q).as_str(), |b| {
        b.iter(|| async_runtime.block_on(async { black_box(bm_quaint(&quaint, q).await) }))
    });

    c.bench_function(format!("pg NoTLS: {}", q).as_str(), |b| {
        b.iter(|| black_box(bm_pg_no_tls(&url, q)))
    });

    let mut client = pg_client(&url);
    c.bench_function(format!("pg NoTLS pre-alloc: {}", q).as_str(), |b| {
        b.iter(|| black_box(bm_pg_no_tls_prealloc_conn(&mut client, q)))
    });

    c.bench_function(format!("pg/tokio pre-allocated conn: {}", q).as_str(), |b| {
        b.iter(|| {
            async_runtime.block_on(async { black_box(bm_pg_tokio_no_tls_prealloc_conn(&async_client, q).await) })
        });
    });
    async_runtime.block_on(async { drop(async_client) });
}

async fn bm_quaint_prealloc_conn(conn: &PooledConnection, query: &str) {
    conn.execute_raw(&query, &[]).await.unwrap();
}

async fn bm_quaint(quaint: &Quaint, query: &str) {
    let conn = quaint.check_out().await.unwrap();
    conn.execute_raw(&query, &[]).await.unwrap();
}

fn bm_pg_no_tls(postgres_url: &str, query: &str) {
    let mut client = Client::connect(postgres_url, NoTls).unwrap();
    client.query(query, &[]).unwrap();
}

fn bm_pg_no_tls_prealloc_conn(client: &mut Client, query: &str) {
    client.query(query, &[]).unwrap();
}

fn pg_client(postgres_url: &str) -> Client {
    Client::connect(postgres_url, NoTls).unwrap()
}

async fn pg_async_client(postgres_url: &str) -> AsyncClient {
    let (client, connection) = tokio_postgres::connect(postgres_url, NoTls).await.unwrap();
    tokio::spawn(async move { connection.await.unwrap() });
    client
}

async fn bm_pg_tokio_no_tls_prealloc_conn(client: &AsyncClient, query: &str) {
    client.query(query, &[]).await.unwrap();
}

async fn quaint_conn(postgres_url: &str) -> PooledConnection {
    async fn get_conn(pool: &Quaint) -> PooledConnection {
        pool.check_out().await.unwrap()
    }

    let mut builder = Quaint::builder(postgres_url).expect("should connect");

    builder.health_check_interval(std::time::Duration::from_secs(15));
    builder.test_on_check_out(true);

    let quaint = builder.build();

    get_conn(&quaint).await
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
