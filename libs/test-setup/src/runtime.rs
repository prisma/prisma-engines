use once_cell::sync::Lazy;

pub fn run_with_tokio<O, F: std::future::Future<Output = O>>(fut: F) -> O {
    test_tokio_runtime().block_on(fut)
}

static RT: Lazy<tokio::runtime::Runtime> = Lazy::new(|| {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap()
});

pub fn run_with_thread_local_runtime<O>(fut: impl std::future::Future<Output = O>) -> O {
    RT.block_on(fut)
}

pub fn test_tokio_runtime() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
}
