use futures::TryFutureExt;
use std::future::Future;

use crate::common::SpawnError;

pub fn spawn_if_possible<F>(future: F) -> impl Future<Output = Result<F::Output, SpawnError>>
where
    F: Future + 'static + Send,
    F::Output: Send + 'static,
{
    tokio::spawn(future).map_err(|_| SpawnError)
}
