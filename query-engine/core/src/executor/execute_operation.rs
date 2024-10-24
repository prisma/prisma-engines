#![allow(unused_variables)]
#![cfg_attr(not(feature = "metrics"), allow(clippy::let_and_return))]

use super::pipeline::QueryPipeline;
use crate::{
    executor::request_context, protocol::EngineProtocol, CoreError, IrSerializer, Operation, QueryGraph,
    QueryGraphBuilder, QueryInterpreter, ResponseData,
};
use connector::{Connection, ConnectionLike, Connector};
use crosstarget_utils::time::ElapsedTimeCounter;
use futures::future;

#[cfg(not(feature = "metrics"))]
use crate::metrics::MetricsInstrumentationStub;
#[cfg(feature = "metrics")]
use query_engine_metrics::{
    counter, histogram, WithMetricsInstrumentation, PRISMA_CLIENT_QUERIES_DURATION_HISTOGRAM_MS,
    PRISMA_CLIENT_QUERIES_TOTAL,
};

use schema::{QuerySchema, QuerySchemaRef};
use std::time::Duration;
use tracing::Instrument;
use tracing_futures::WithSubscriber;

pub async fn execute_single_operation(
    query_schema: QuerySchemaRef,
    conn: &mut dyn ConnectionLike,
    operation: &Operation,
    trace_id: Option<String>,
) -> crate::Result<ResponseData> {
    let operation_timer = ElapsedTimeCounter::start();

    let (graph, serializer) = build_graph(&query_schema, operation.clone())?;
    let result = execute_on(conn, graph, serializer, query_schema.as_ref(), trace_id).await;

    #[cfg(feature = "metrics")]
    histogram!(PRISMA_CLIENT_QUERIES_DURATION_HISTOGRAM_MS).record(operation_timer.elapsed_time());

    result
}

pub async fn execute_many_operations(
    query_schema: QuerySchemaRef,
    conn: &mut dyn ConnectionLike,
    operations: &[Operation],
    trace_id: Option<String>,
) -> crate::Result<Vec<crate::Result<ResponseData>>> {
    let queries = operations
        .iter()
        .map(|operation| build_graph(&query_schema, operation.clone()))
        .collect::<std::result::Result<Vec<_>, _>>()?;

    let mut results = Vec::with_capacity(queries.len());

    for (i, (graph, serializer)) in queries.into_iter().enumerate() {
        let operation_timer = ElapsedTimeCounter::start();
        let result = execute_on(conn, graph, serializer, query_schema.as_ref(), trace_id.clone()).await;

        #[cfg(feature = "metrics")]
        histogram!(PRISMA_CLIENT_QUERIES_DURATION_HISTOGRAM_MS).record(operation_timer.elapsed_time());

        match result {
            Ok(result) => results.push(Ok(result)),
            Err(error) => {
                return Err(crate::CoreError::BatchError {
                    request_idx: i,
                    error: Box::new(error),
                });
            }
        }
    }

    Ok(results)
}

pub async fn execute_single_self_contained<C: Connector + Send + Sync>(
    connector: &C,
    query_schema: QuerySchemaRef,
    operation: Operation,
    trace_id: Option<String>,
    force_transactions: bool,
) -> crate::Result<ResponseData> {
    let conn_span = info_span!(
        "prisma:engine:connection",
        user_facing = true,
        "db.system" = connector.name(),
    );
    let conn = connector.get_connection().instrument(conn_span).await?;

    execute_self_contained(
        conn,
        query_schema,
        operation,
        force_transactions,
        connector.should_retry_on_transient_error(),
        trace_id,
    )
    .await
}

pub async fn execute_many_self_contained<C: Connector + Send + Sync>(
    connector: &C,
    query_schema: QuerySchemaRef,
    operations: &[Operation],
    trace_id: Option<String>,
    force_transactions: bool,
    engine_protocol: EngineProtocol,
) -> crate::Result<Vec<crate::Result<ResponseData>>> {
    let mut futures = Vec::with_capacity(operations.len());

    for op in operations {
        #[cfg(feature = "metrics")]
        counter!(PRISMA_CLIENT_QUERIES_TOTAL).increment(1);

        let conn_span = info_span!(
            "prisma:engine:connection",
            user_facing = true,
            "db.system" = connector.name(),
        );
        let conn = connector.get_connection().instrument(conn_span).await?;

        futures.push(crosstarget_utils::spawn::spawn_if_possible(
            request_context::with_request_context(
                engine_protocol,
                execute_self_contained(
                    conn,
                    query_schema.clone(),
                    op.clone(),
                    force_transactions,
                    connector.should_retry_on_transient_error(),
                    trace_id.clone(),
                ),
            )
            .with_current_subscriber()
            .with_current_recorder(),
        ));
    }

    let responses: Vec<_> = future::join_all(futures)
        .await
        .into_iter()
        .map(|res| res.expect("IO Error in tokio::spawn"))
        .collect();

    Ok(responses)
}

/// Execute the operation as a self-contained operation, if necessary wrapped in a transaction.
async fn execute_self_contained(
    mut conn: Box<dyn Connection>,
    query_schema: QuerySchemaRef,
    operation: Operation,
    force_transactions: bool,
    retry_on_transient_error: bool,
    trace_id: Option<String>,
) -> crate::Result<ResponseData> {
    let operation_timer = ElapsedTimeCounter::start();
    let result = if retry_on_transient_error {
        execute_self_contained_with_retry(
            &mut conn,
            query_schema,
            operation,
            force_transactions,
            ElapsedTimeCounter::start(),
            trace_id,
        )
        .await
    } else {
        let (graph, serializer) = build_graph(&query_schema, operation)?;

        execute_self_contained_without_retry(conn, graph, serializer, force_transactions, &query_schema, trace_id).await
    };

    #[cfg(feature = "metrics")]
    histogram!(PRISMA_CLIENT_QUERIES_DURATION_HISTOGRAM_MS).record(operation_timer.elapsed_time());

    result
}

async fn execute_self_contained_without_retry<'a>(
    mut conn: Box<dyn Connection>,
    graph: QueryGraph,
    serializer: IrSerializer<'a>,
    force_transactions: bool,
    query_schema: &'a QuerySchema,
    trace_id: Option<String>,
) -> crate::Result<ResponseData> {
    if force_transactions || graph.needs_transaction() {
        return execute_in_tx(&mut conn, graph, serializer, query_schema, trace_id).await;
    }

    execute_on(conn.as_connection_like(), graph, serializer, query_schema, trace_id).await
}

// As suggested by the MongoDB documentation
// https://github.com/mongodb/specifications/blob/master/source/transactions-convenient-api/transactions-convenient-api.rst#pseudo-code
const MAX_TX_TIMEOUT_RETRY_LIMIT: Duration = Duration::from_secs(12);
const TX_RETRY_BACKOFF: Duration = Duration::from_millis(5);

// MongoDB-specific transient transaction error retry logic.
// Hack: This should ideally live in MongoDb's connector but our current architecture doesn't allow us to easily do that.
async fn execute_self_contained_with_retry(
    conn: &mut Box<dyn Connection>,
    query_schema: QuerySchemaRef,
    operation: Operation,
    force_transactions: bool,
    retry_timeout: ElapsedTimeCounter,
    trace_id: Option<String>,
) -> crate::Result<ResponseData> {
    let (graph, serializer) = build_graph(&query_schema, operation.clone())?;

    if force_transactions || graph.needs_transaction() {
        let res = execute_in_tx(conn, graph, serializer, query_schema.as_ref(), trace_id.clone()).await;

        if !is_transient_error(&res) {
            return res;
        }

        loop {
            let (graph, serializer) = build_graph(&query_schema, operation.clone())?;
            let res = execute_in_tx(conn, graph, serializer, query_schema.as_ref(), trace_id.clone()).await;

            if is_transient_error(&res) && retry_timeout.elapsed_time() < MAX_TX_TIMEOUT_RETRY_LIMIT {
                crosstarget_utils::time::sleep(TX_RETRY_BACKOFF).await;
                continue;
            } else {
                return res;
            }
        }
    } else {
        execute_on(
            conn.as_connection_like(),
            graph,
            serializer,
            query_schema.as_ref(),
            trace_id,
        )
        .await
    }
}

async fn execute_in_tx<'a>(
    conn: &mut Box<dyn Connection>,
    graph: QueryGraph,
    serializer: IrSerializer<'a>,
    query_schema: &'a QuerySchema,
    trace_id: Option<String>,
) -> crate::Result<ResponseData> {
    let mut tx = conn.start_transaction(None).await?;
    let result = execute_on(
        tx.as_connection_like(),
        graph,
        serializer,
        query_schema,
        trace_id.clone(),
    )
    .await;

    if result.is_ok() {
        tx.commit().await?;
    } else {
        tx.rollback().await?;
    }

    result
}

// Simplest execution on anything that's a ConnectionLike. Caller decides handling of connections and transactions.
async fn execute_on<'a>(
    conn: &mut dyn ConnectionLike,
    graph: QueryGraph,
    serializer: IrSerializer<'a>,
    query_schema: &'a QuerySchema,
    trace_id: Option<String>,
) -> crate::Result<ResponseData> {
    #[cfg(feature = "metrics")]
    counter!(PRISMA_CLIENT_QUERIES_TOTAL).increment(1);

    let interpreter = QueryInterpreter::new(conn);
    QueryPipeline::new(graph, interpreter, serializer)
        .execute(query_schema, trace_id)
        .await
}

fn build_graph(query_schema: &QuerySchema, operation: Operation) -> crate::Result<(QueryGraph, IrSerializer<'_>)> {
    let (query_graph, serializer) = QueryGraphBuilder::new(query_schema).build(operation)?;

    Ok((query_graph, serializer))
}

fn is_transient_error<T>(res: &Result<T, CoreError>) -> bool {
    match res {
        Ok(_) => false,
        Err(err) => err.is_transient(),
    }
}
