use std::future::Future;

use crate::common::SpawnError;

pub async fn spawn_if_possible<F>(future: F) -> Result<F::Output, SpawnError>
where
    F: Future + 'static + Send,
    F::Output: Send + 'static,
{
    tokio::spawn(future).await.map_err(|_| SpawnError)
}
