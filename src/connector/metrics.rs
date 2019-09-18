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
    if *crate::LOG_QUERIES {
        info!(
            "query: \"{}\", params: {}",
            query,
            Params(params)
        );
    }

    time(format!("{}.query.time ({})", tag, query), f)
}

pub(crate) fn connect<F, T>(tag: &'static str, f: F) -> T
where
    F: FnOnce() -> T,
{
    time(format!("{}.connect.time", tag), f)
}

fn time<F, T>(tag: String, f: F) -> T
where
    F: FnOnce() -> T,
{
    let start = Instant::now();
    let res = f();
    let end = Instant::now();

    timing!(tag, start, end);

    res
}
