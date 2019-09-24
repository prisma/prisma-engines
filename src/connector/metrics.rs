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
        info!(
            "query: \"{}\", params: {} (in {}ms)",
            query,
            Params(params),
            start.elapsed().as_millis(),
        );
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
