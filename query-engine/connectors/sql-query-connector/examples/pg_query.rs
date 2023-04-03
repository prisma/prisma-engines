use std::time::Instant;

use quaint::{
    pooled::{PooledConnection, Quaint},
    prelude::Queryable,
};

#[tokio::main]
async fn main() -> () {
    let url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set to a postgres URL");
    let pool = quaint(&url);

    for i in 0..1 {
        let now = Instant::now();
        bm_quaint(&pool, "SELECT 1").await;

        let elapsed = now.elapsed();
        println!("{i}. Elapsed: {:?}", elapsed);
    }
}

async fn bm_quaint_prealloc_conn(conn: &PooledConnection, query: &str) -> u64 {
    let res = conn.execute_raw(&query, &[]).await.unwrap();

    res
}

async fn bm_quaint(quaint: &Quaint, query: &str) -> u64 {
    let conn = quaint_conn(quaint).await;

    bm_quaint_prealloc_conn(&conn, &query).await
}

fn quaint(url: &str) -> Quaint {
    let mut builder = Quaint::builder(url).expect("should connect");

    builder.health_check_interval(std::time::Duration::from_secs(15));
    builder.test_on_check_out(true);

    builder.build()
}

async fn quaint_conn(quaint: &Quaint) -> PooledConnection {
    async fn get_conn(pool: &Quaint) -> PooledConnection {
        pool.check_out().await.unwrap()
    }

    get_conn(&quaint).await
}
