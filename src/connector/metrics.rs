use crate::ast::{Params, Value};
use std::{future::Future, time::Instant};

pub(crate) async fn query<'a, F, T, U>(
    tag: &'static str,
    query: &'a str,
    params: &'a [Value<'_>],
    f: F,
) -> crate::Result<T>
where
    F: FnOnce() -> U + 'a,
    U: Future<Output = crate::Result<T>>,
{
    let start = Instant::now();
    let res = f().await;
    let end = Instant::now();

    if *crate::LOG_QUERIES {
        let result = match res {
            Ok(_) => "success",
            Err(_) => "error",
        };

        tracing::info!(
            query,
            item_type = "query",
            params = %Params(params),
            duration_ms = start.elapsed().as_millis() as u64,
            result,
        )
    }

    timing!(format!("{}.query.time", tag), start, end);

    res
}

#[cfg(feature = "pooled")]
pub(crate) async fn check_out<F, T>(f: F) -> std::result::Result<T, mobc::Error<crate::error::Error>>
where
    F: Future<Output = std::result::Result<T, mobc::Error<crate::error::Error>>>,
{
    let start = Instant::now();
    let res = f.await;
    let end = Instant::now();

    if *crate::LOG_QUERIES {
        let result = match res {
            Ok(_) => "success",
            Err(_) => "error",
        };

        tracing::info!(
            message = "Fetched a connection from the pool",
            duration_ms = start.elapsed().as_millis() as u64,
            result,
        );
    }

    timing!("pool.check_out", start, end);

    res
}
