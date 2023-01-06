use std::time::Instant;

use super::pipeline::QueryPipeline;
use crate::{IrSerializer, Operation, QueryGraph, QueryGraphBuilder, QueryInterpreter, ResponseData};
use connector::{Connection, ConnectionLike, Connector};
use futures::future;
use query_engine_metrics::{
    histogram, increment_counter, metrics, PRISMA_CLIENT_QUERIES_HISTOGRAM_MS, PRISMA_CLIENT_QUERIES_TOTAL,
};
use schema::QuerySchemaRef;
use tracing::Instrument;
use tracing_futures::WithSubscriber;

pub async fn execute_single_operation(
    query_schema: QuerySchemaRef,
    conn: &mut dyn ConnectionLike,
    operation: &Operation,
    trace_id: Option<String>,
) -> crate::Result<ResponseData> {
    let operation_timer = Instant::now();
    let interpreter = QueryInterpreter::new(conn);
    let (query_graph, serializer) = QueryGraphBuilder::new(query_schema.clone()).build(operation.clone())?;

    increment_counter!(PRISMA_CLIENT_QUERIES_TOTAL);

    let result = QueryPipeline::new(query_graph, interpreter, serializer)
        .execute(trace_id)
        .await;

    histogram!(PRISMA_CLIENT_QUERIES_HISTOGRAM_MS, operation_timer.elapsed());
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
        .map(|operation| QueryGraphBuilder::new(query_schema.clone()).build(operation.clone()))
        .collect::<std::result::Result<Vec<_>, _>>()?;

    let mut results = Vec::with_capacity(queries.len());

    for (i, (query_graph, serializer)) in queries.into_iter().enumerate() {
        increment_counter!(PRISMA_CLIENT_QUERIES_TOTAL);
        let operation_timer = Instant::now();
        let interpreter = QueryInterpreter::new(conn);
        let result = QueryPipeline::new(query_graph, interpreter, serializer)
            .execute(trace_id.clone())
            .await;

        histogram!(PRISMA_CLIENT_QUERIES_HISTOGRAM_MS, operation_timer.elapsed());

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
    let (query_graph, serializer) = QueryGraphBuilder::new(query_schema).build(operation)?;
    let conn_span = info_span!(
        "prisma:engine:connection",
        user_facing = true,
        "db.type" = connector.name()
    );
    let conn = connector.get_connection().instrument(conn_span).await?;
    execute_self_contained(conn, query_graph, serializer, force_transactions, trace_id).await
}

pub async fn execute_many_self_contained<C: Connector + Send + Sync>(
    connector: &C,
    query_schema: QuerySchemaRef,
    operations: &[Operation],
    trace_id: Option<String>,
    force_transactions: bool,
) -> crate::Result<Vec<crate::Result<ResponseData>>> {
    let mut futures = Vec::with_capacity(operations.len());

    let dispatcher = crate::get_current_dispatcher();
    for op in operations {
        match QueryGraphBuilder::new(query_schema.clone()).build(op.clone()) {
            Ok((graph, serializer)) => {
                increment_counter!(PRISMA_CLIENT_QUERIES_TOTAL);

                let conn_span = info_span!(
                    "prisma:engine:connection",
                    user_facing = true,
                    "db.type" = connector.name(),
                );
                let conn = connector.get_connection().instrument(conn_span).await?;

                futures.push(tokio::spawn(
                    execute_self_contained(conn, graph, serializer, force_transactions, trace_id.clone())
                        .with_subscriber(dispatcher.clone()),
                ));
            }

            // This looks unnecessary, but is the simplest way to preserve ordering of results for the batch.
            Err(err) => futures.push(tokio::spawn(async move { Err(err.into()) })),
        }
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
    graph: QueryGraph,
    serializer: IrSerializer,
    force_transactions: bool,
    trace_id: Option<String>,
) -> crate::Result<ResponseData> {
    let operation_timer = Instant::now();
    let result = if force_transactions || graph.needs_transaction() {
        let mut tx = conn.start_transaction(None).await?;
        let result = execute_on(tx.as_connection_like(), graph, serializer, trace_id).await;

        if result.is_ok() {
            tx.commit().await?;
        } else {
            tx.rollback().await?;
        }

        result
    } else {
        execute_on(conn.as_connection_like(), graph, serializer, trace_id).await
    };

    histogram!(PRISMA_CLIENT_QUERIES_HISTOGRAM_MS, operation_timer.elapsed());
    result
}

// Simplest execution on anything that's a ConnectionLike. Caller decides handling of connections and transactions.
async fn execute_on(
    conn: &mut dyn ConnectionLike,
    graph: QueryGraph,
    serializer: IrSerializer,
    trace_id: Option<String>,
) -> crate::Result<ResponseData> {
    increment_counter!(PRISMA_CLIENT_QUERIES_TOTAL);
    let interpreter = QueryInterpreter::new(conn);
    QueryPipeline::new(graph, interpreter, serializer)
        .execute(trace_id)
        .await
}
