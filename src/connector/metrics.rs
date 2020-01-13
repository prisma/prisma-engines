use crate::{
    ast::{ParameterizedValue, Params},
    connector::DBIO,
};
use std::{future::Future, time::Instant};

pub(crate) fn query<'a, F, T, U>(
    tag: &'static str,
    query: &'a str,
    params: &'a [ParameterizedValue],
    f: F,
) -> DBIO<'a, T>
where
    F: FnOnce() -> U + Send + 'a,
    U: Future<Output = crate::Result<T>> + Send,
{
    DBIO::new(async move {
        let start = Instant::now();
        let res = f().await;
        let end = Instant::now();

        if *crate::LOG_QUERIES {
            #[cfg(not(feature = "tracing-log"))]
            {
                info!(
                    "query: \"{}\", params: {} (in {}ms)",
                    query,
                    Params(params),
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
                )
            }
        }

        timing!(format!("{}.query.time", tag), start, end);

        res
    })
}
