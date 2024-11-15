use std::future::Future;

use futures::TryFutureExt;
use tokio::sync::oneshot;
use wasm_bindgen_futures::spawn_local;

use crate::common::spawn::SpawnError;

pub fn spawn_if_possible<F>(future: F) -> impl Future<Output = Result<F::Output, SpawnError>>
where
    F: Future + 'static,
{
    let (sx, rx) = oneshot::channel();
    spawn_local(async move {
        let result = future.await;
        _ = sx.send(result);
    });

    rx.map_err(|_| SpawnError)
}
