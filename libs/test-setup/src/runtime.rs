pub fn run_with_tokio<O, F: std::future::Future<Output = O>>(fut: F) -> O {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(fut)
}
