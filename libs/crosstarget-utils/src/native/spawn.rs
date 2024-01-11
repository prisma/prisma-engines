use futures::FutureExt;
use std::future::Future;

use crate::common::SpawnError;

pub fn spawn_if_possible<F>(future: F) -> impl Future<Output = Result<F::Output, SpawnError>>
where
    F: Future + 'static + Send,
    F::Output: Send + 'static,
{
    tokio::spawn(future).map(|result| match result {
        Ok(result) => Ok(result),
        Err(_) => Err(SpawnError),
    })
}
