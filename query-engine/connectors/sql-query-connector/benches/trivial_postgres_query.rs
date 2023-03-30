use codspeed_criterion_compat::{black_box, criterion_group, criterion_main, Criterion};
use postgres::{Client, NoTls};
use quaint::{
    pooled::{PooledConnection, Quaint},
    prelude::Queryable,
};
use tokio_postgres::Client as AsyncClient;

fn trivial_query_benchmark(c: &mut Criterion) {
    let url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set to a postgres URL");
    let queries = vec!["SELECT 1"];

    tokio::runtime::Runtime::new().unwrap().block_on(async {
        for q in &queries {
            let conn = quaint_conn(&url).await;
            c.bench_function(format!("Quaint pre-allocated conn: {}", q).as_str(), |b| {
                b.iter(|| async { bm_quaint_prealloc_conn(&conn, q).await })
            });

            c.bench_function(format!("Quaint: {}", q).as_str(), |b| {
                b.iter(|| async { black_box(bm_quaint(&url, q)).await })
            });

            let client = pg_async_client(&url).await;
            c.bench_function(format!("pg-tokio NoTLS: {}", q).as_str(), |b| {
                b.iter(|| async { black_box(bm_pg_tokio_no_tls_prealloc_conn(&client, q)).await })
            });
        }
    });

    for q in queries {
        c.bench_function(format!("pg NoTLS: {}", q).as_str(), |b| {
            b.iter(|| black_box(bm_pg_no_tls(&url, q)))
        });

        let mut client = pg_client(&url);
        c.bench_function(format!("pg NoTLS pre-alloc: {}", q).as_str(), |b| {
            b.iter(|| black_box(bm_pg_no_tls_prealloc_conn(&mut client, q)))
        });
    }
}

#[inline(always)]
async fn bm_quaint_prealloc_conn(conn: &PooledConnection, query: &str) {
    conn.execute_raw(&query, &[]).await.unwrap();
}

#[inline(always)]
async fn bm_quaint(postgres_url: &str, query: &str) {
    let conn = quaint_conn(postgres_url).await;
    bm_quaint_prealloc_conn(&conn, &query).await;
}

#[inline(always)]
fn bm_pg_no_tls(postgres_url: &str, query: &str) {
    let mut client = Client::connect(postgres_url, NoTls).unwrap();
    client.query(query, &[]).unwrap();
}

#[inline(always)]
fn bm_pg_no_tls_prealloc_conn(client: &mut Client, query: &str) {
    client.query(query, &[]).unwrap();
}

#[inline(always)]
fn pg_client(postgres_url: &str) -> Client {
    Client::connect(postgres_url, NoTls).unwrap()
}

#[inline(always)]
async fn pg_async_client(postgres_url: &str) -> AsyncClient {
    let (client, connection) = tokio_postgres::connect(postgres_url, NoTls).await.unwrap();
    tokio::spawn(async move { connection.await.unwrap() });
    client
}

#[inline(always)]
async fn bm_pg_tokio_no_tls_prealloc_conn(client: &AsyncClient, query: &str) {
    client.query(query, &[]).await.unwrap();
}

#[inline(always)]
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

criterion_group!(benches, trivial_query_benchmark);
criterion_main!(benches);
