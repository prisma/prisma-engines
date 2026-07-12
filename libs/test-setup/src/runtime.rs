use std::sync::LazyLock;

static RT: LazyLock<tokio::runtime::Runtime> = LazyLock::new(|| {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap()
});

pub fn run_with_thread_local_runtime<O>(fut: impl std::future::Future<Output = O>) -> O {
    RT.block_on(fut)
}
