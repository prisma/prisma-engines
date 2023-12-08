use std::future::Future;

use crate::common::SpawnError;

pub async fn spawn_if_possible<F>(future: F) -> Result<F::Output, SpawnError>
where
    F: Future + 'static,
{
    Ok(future.await)
}
