use std::future::Future;

use futures::FutureExt;

use crate::common::SpawnError;

pub fn spawn_if_possible<F>(future: F) -> impl Future<F::Output, SpawnError>
where
    F: Future + 'static,
{
    future.map(Ok)
}
