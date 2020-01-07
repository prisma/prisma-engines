pub fn run_with_tokio<O, F: std::future::Future<Output = O>>(fut: F) -> O {
    tokio::runtime::Builder::new()
        .basic_scheduler()
        .enable_all()
        .build()
        .unwrap()
        .block_on(fut)
}
