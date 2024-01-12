use std::future::Future;

use futures::FutureExt;
use tokio::sync::oneshot;
use wasm_bindgen_futures::spawn_local;

use crate::common::SpawnError;

pub fn spawn_if_possible<F>(future: F) -> impl Future<Output = Result<F::Output, SpawnError>>
where
    F: Future + 'static,
{
    let (sx, rx) = oneshot::channel::<F::Output>();
    spawn_local(async move {
        let result = future.await;
        let _ = sx.send(result);
    });

    rx.map(|result| match result {
        Ok(result) => Ok(result),
        Err(_) => Err(SpawnError),
    })
}
