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

        #[cfg(not(feature = "tracing-log"))]
        {
            info!(
                "query: \"{}\", params: {} ({} in {}ms)",
                query,
                Params(params),
                result,
                start.elapsed().as_millis(),
            );
        }
        #[cfg(feature = "tracing-log")]
        {
            tracing::info!(
                query,
                item_type = "query",
                params = %Params(params),
                duration_ms = start.elapsed().as_millis() as u64,
                result,
            )
        }
    }

    timing!(format!("{}.query.time", tag), start, end);

    res
}
