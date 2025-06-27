use quaint::connector::tokio_postgres;
use std::time::Instant;

#[tokio::main]
async fn main() {
    let url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set to a postgres URL");
    let start = Instant::now();

    let now = Instant::now();
    let (client, connection) = tokio_postgres::connect(&url, tokio_postgres::NoTls).await.unwrap();
    println!("Connect: {:?}", now.elapsed());

    tokio::spawn(async move {
        let now = Instant::now();
        if let Err(e) = connection.await {
            eprintln!("connection error: {e}");
        }
        eprintln!("connection time: {:?}", now.elapsed());
    });

    let now = Instant::now();
    client.query("SELECT 1;", &[]).await.unwrap();
    println!("Query: {:?}", now.elapsed());

    println!("Total: {:?}", start.elapsed());
}
