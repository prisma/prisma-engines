pub fn run_with_tokio<O, F: std::future::Future<Output = O>>(fut: F) -> O {
    test_tokio_runtime().block_on(fut)
}

std::thread_local! {
    static RT: tokio::runtime::Runtime = test_tokio_runtime();
}

pub fn run_with_thread_local_runtime<O>(fut: impl std::future::Future<Output = O>) -> O {
    RT.with(|rt| rt.block_on(fut))
}

pub fn test_tokio_runtime() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
}
