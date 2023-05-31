use quaint::connector::mysql_async::{self, prelude::*};
use std::time::Instant;

#[tokio::main]
async fn main() {
    let url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set to a mysql URL");
    let start = Instant::now();

    let now = Instant::now();
    let pool = mysql_async::Pool::from_url(&url).unwrap();
    println!("Building pool: {:?}", now.elapsed());

    let mut conn = pool.get_conn().await.unwrap();
    println!("Checking out connection: {:?}", now.elapsed());

    let now = Instant::now();
    let _: Vec<u8> = conn.query("SELECT 1").await.unwrap();
    println!("Query: {:?}", now.elapsed());

    println!("Total: {:?}", start.elapsed());
}
