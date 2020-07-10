pub fn with_runtime<O, F: std::future::Future<Output = O>>(fut: F) -> O {
    async_std::task::block_on(fut)
}
