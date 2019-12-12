pub fn run_with_tokio<T, F: std::future::Future<Output = T>>(fut: F) -> T {
    tokio::runtime::Builder::new()
        .basic_scheduler()
        .enable_all()
        .build()
        .unwrap()
        .block_on(fut)
}
