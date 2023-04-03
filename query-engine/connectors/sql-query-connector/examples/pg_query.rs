use std::time::Instant;

use postgres::{Client, NoTls};
use quaint::{
    pooled::{PooledConnection, Quaint},
    prelude::Queryable,
};
use tokio_postgres::Client as AsyncClient;

#[tokio::main]
async fn main() -> () {
    let url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set to a postgres URL");
    let queries = vec!["SELECT 1"];

    for q in &queries {
        let now = Instant::now();
        let res = bm_quaint(&url, q).await;
        let elapsed = now.elapsed();

        println!("Elapsed: {:?}", elapsed);
    }
}

async fn bm_quaint_prealloc_conn(conn: &PooledConnection, query: &str) -> u64 {
    let res = conn.execute_raw(&query, &[]).await.unwrap();

    res
}

async fn bm_quaint(postgres_url: &str, query: &str) -> u64 {
    let conn = quaint_conn(postgres_url).await;

    bm_quaint_prealloc_conn(&conn, &query).await
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
