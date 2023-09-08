use quaint::{pooled::Quaint, prelude::Queryable};
use std::time::Instant;

#[tokio::main]
async fn main() {
    let url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set to a quaint-compatible database URL");
    let start = Instant::now();

    let now = Instant::now();
    let mut builder = Quaint::builder(&url).expect("should connect");
    builder.health_check_interval(std::time::Duration::from_secs(15));
    builder.test_on_check_out(true);
    let pool = builder.build();
    println!("Building pool: {:?}", now.elapsed());

    let now = Instant::now();
    let conn = pool.check_out().await.unwrap();
    println!("Checking out connection: {:?}", now.elapsed());

    let now = Instant::now();
    conn.execute_raw("SELECT 1", &[]).await.unwrap();
    println!("Query: {:?}", now.elapsed());

    println!("Total: {:?}", start.elapsed());
}
