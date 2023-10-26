use tracing::{info_span, Instrument};

use crate::ast::{Params, Value};
use std::{future::Future, time::Instant};

pub async fn query<'a, F, T, U>(tag: &'static str, query: &'a str, params: &'a [Value<'_>], f: F) -> crate::Result<T>
where
    F: FnOnce() -> U + 'a,
    U: Future<Output = crate::Result<T>>,
{
    let span = info_span!("quaint:query", "db.statement" = %query);
    do_query(tag, query, params, f).instrument(span).await
}

async fn do_query<'a, F, T, U>(tag: &'static str, query: &'a str, params: &'a [Value<'_>], f: F) -> crate::Result<T>
where
    F: FnOnce() -> U + 'a,
    U: Future<Output = crate::Result<T>>,
{
    let start = Instant::now();
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

            trace_query(&query_fmt, params, result, start);
        } else {
            trace_query(query, params, result, start);
        };
    }

    #[cfg(not(feature = "fmt-sql"))]
    {
        trace_query(query, params, result, start);
    }

    histogram!(format!("{tag}.query.time"), start.elapsed());
    histogram!("prisma_datasource_queries_duration_histogram_ms", start.elapsed());
    increment_counter!("prisma_datasource_queries_total");

    res
}

#[cfg(feature = "pooled")]
pub(crate) async fn check_out<F, T>(f: F) -> std::result::Result<T, mobc::Error<crate::error::Error>>
where
    F: Future<Output = std::result::Result<T, mobc::Error<crate::error::Error>>>,
{
    let start = Instant::now();
    let res = f.await;

    let result = match res {
        Ok(_) => "success",
        Err(_) => "error",
    };

    tracing::trace!(
        message = "Fetched a connection from the pool",
        duration_ms = start.elapsed().as_millis() as u64,
        item_type = "query",
        is_query = true,
        result,
    );

    histogram!("pool.check_out", start.elapsed());

    res
}

fn trace_query<'a>(query: &'a str, params: &'a [Value<'_>], result: &str, start: Instant) {
    tracing::debug!(
        query = %query,
        params = %Params(params),
        result,
        item_type = "query",
        is_query = true,
        duration_ms = start.elapsed().as_millis() as u64,
    );
}
