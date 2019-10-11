use crate::ast::{ParameterizedValue, Params};
use std::time::Instant;

pub(crate) fn query<'a, F, T>(
    tag: &'static str,
    query: &str,
    params: &[ParameterizedValue<'a>],
    f: F,
) -> T
where
    F: FnOnce() -> T,
{
    let start = Instant::now();
    let res = f();
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
            //let params: Vec<String> = params.iter().map(|p| format!("{}", p)).collect();

            tracing::info!(
                query,
                params = %Params(params),
                duration_ns = start.elapsed().as_nanos() as u64,
            )
        }
    }

    timing!(format!("{}.query.time", tag), start, end);

    res
}

pub(crate) fn connect<F, T>(tag: &'static str, f: F) -> T
where
    F: FnOnce() -> T,
{
    let start = Instant::now();
    let res = f();
    let end = Instant::now();

    timing!(format!("{}.connect.time", tag), start, end);

    res
}
