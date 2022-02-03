use super::pipeline::QueryPipeline;
use crate::{IrSerializer, Operation, QueryGraph, QueryGraphBuilder, QueryInterpreter, QuerySchemaRef, ResponseData};
use connector::{Connection, ConnectionLike, Connector};
use futures::future;

pub async fn execute_single_operation(
    query_schema: QuerySchemaRef,
    conn: &mut dyn ConnectionLike,
    operation: &Operation,
    trace_id: Option<String>,
) -> crate::Result<ResponseData> {
    let interpreter = QueryInterpreter::new(conn);
    let (query_graph, serializer) = QueryGraphBuilder::new(query_schema.clone()).build(operation.clone())?;

    QueryPipeline::new(query_graph, interpreter, serializer)
        .execute(trace_id)
        .await
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

    for (query_graph, serializer) in queries {
        let interpreter = QueryInterpreter::new(conn);
        let result = QueryPipeline::new(query_graph, interpreter, serializer)
            .execute(trace_id.clone())
            .await?;

        results.push(Ok(result));
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
    let conn = connector.get_connection().await?;
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

    for op in operations {
        match QueryGraphBuilder::new(query_schema.clone()).build(op.clone()) {
            Ok((graph, serializer)) => {
                let conn = connector.get_connection().await?;

                futures.push(tokio::spawn(execute_self_contained(
                    conn,
                    graph,
                    serializer,
                    force_transactions,
                    trace_id.clone(),
                )));
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
#[tracing::instrument(skip(conn, graph, serializer, force_transactions))]
async fn execute_self_contained(
    mut conn: Box<dyn Connection>,
    graph: QueryGraph,
    serializer: IrSerializer,
    force_transactions: bool,
    trace_id: Option<String>,
) -> crate::Result<ResponseData> {
    if force_transactions || graph.needs_transaction() {
        let mut tx = conn.start_transaction().await?;
        let result = execute_on(tx.as_connection_like(), graph, serializer, trace_id).await;

        if result.is_ok() {
            tx.commit().await?;
        } else {
            tx.rollback().await?;
        }

        result
    } else {
        execute_on(conn.as_connection_like(), graph, serializer, trace_id).await
    }
}

// Simplest execution on anything that's a ConnectionLike. Caller decides handling of connections and transactions.
async fn execute_on(
    conn: &mut dyn ConnectionLike,
    graph: QueryGraph,
    serializer: IrSerializer,
    trace_id: Option<String>,
) -> crate::Result<ResponseData> {
    let interpreter = QueryInterpreter::new(conn);
    let result = QueryPipeline::new(graph, interpreter, serializer)
        .execute(trace_id)
        .await;

    result
}
