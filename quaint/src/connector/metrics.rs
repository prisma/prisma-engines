use std::future::Future;

use crosstarget_utils::time::ElapsedTimeCounter;
use telemetry::formatting::QueryForTracing;
use tracing::{Instrument, info_span};

use crate::ast::{Params, Value};

pub async fn query<'a, F, T, U>(
    tag: &'static str,
    db_system_name: &'static str,
    query: &'a str,
    params: &'a [Value<'_>],
    f: F,
) -> crate::Result<T>
where
    F: FnOnce() -> U + 'a,
    U: Future<Output = crate::Result<T>>,
{
    let span = info_span!(
        "quaint:query",
        "db.system" = db_system_name,
        "db.query.text" = %QueryForTracing(query),
        "otel.kind" = "client",
        "otel.name" = "prisma:engine:db_query",
        user_facing = true,
    );
    do_query(tag, query, params, f).instrument(span).await
}

async fn do_query<'a, F, T, U>(_tag: &'static str, query: &'a str, params: &'a [Value<'_>], f: F) -> crate::Result<T>
where
    F: FnOnce() -> U + 'a,
    U: Future<Output = crate::Result<T>>,
{
    let start = ElapsedTimeCounter::start();
    let res = f().await;

    let result = match res {
        Ok(_) => "success",
        Err(_) => "error",
    };

    #[cfg(feature = "fmt-sql")]
    {
        if std::env::var("FMT_SQL").is_ok() {
            let query_fmt = sqlformat::format(
                query,
                &sqlformat::QueryParams::None,
                sqlformat::FormatOptions::default(),
            );

            trace_query(&query_fmt, params, result, &start);
        } else {
            trace_query(query, params, result, &start);
        };
    }

    #[cfg(not(feature = "fmt-sql"))]
    {
        trace_query(query, params, result, &start);
    }

    res
}

#[cfg(feature = "pooled")]
pub(crate) async fn check_out<F, T>(f: F) -> std::result::Result<T, mobc::Error<crate::error::Error>>
where
    F: Future<Output = std::result::Result<T, mobc::Error<crate::error::Error>>>,
{
    let start = ElapsedTimeCounter::start();
    let res = f.await;

    let result = match res {
        Ok(_) => "success",
        Err(_) => "error",
    };

    tracing::trace!(
        message = "Fetched a connection from the pool",
        duration_ms = start.elapsed_time().as_millis() as u64,
        is_query = true,
        result,
    );

    res
}

fn trace_query<'a>(query: &'a str, params: &'a [Value<'_>], result: &str, start: &ElapsedTimeCounter) {
    tracing::debug!(
        query = %query,
        params = %Params(params),
        result,
        item_type = "query",
        is_query = true,
        duration_ms = start.elapsed_time().as_millis() as u64,
    );
}
