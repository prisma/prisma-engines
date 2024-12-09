use prisma_metrics::{counter, histogram};
use tracing::{info_span, Instrument};

use crate::ast::{Params, Value};
use crosstarget_utils::time::ElapsedTimeCounter;
use std::{fmt, future::Future};

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
        "db.statement" = %QueryForTracing(query),
        "otel.kind" = "client"
    );
    do_query(tag, query, params, f).instrument(span).await
}

async fn do_query<'a, F, T, U>(tag: &'static str, query: &'a str, params: &'a [Value<'_>], f: F) -> crate::Result<T>
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

    histogram!(format!("{tag}.query.time")).record(start.elapsed_time());
    histogram!("prisma_datasource_queries_duration_histogram_ms").record(start.elapsed_time());
    counter!("prisma_datasource_queries_total").increment(1);

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
        item_type = "query",
        is_query = true,
        result,
    );

    histogram!("pool.check_out").record(start.elapsed_time());

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

struct QueryForTracing<'a>(&'a str);

impl fmt::Display for QueryForTracing<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let query = self
            .0
            .split_once("/* traceparent=")
            .map_or(self.0, |(str, remainder)| {
                if remainder
                    .split_once("*/")
                    .is_some_and(|(_, suffix)| suffix.trim_end().is_empty())
                {
                    str
                } else {
                    self.0
                }
            })
            .trim();
        write!(f, "{query}")
    }
}
