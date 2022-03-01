use crate::{
    interpreter::{InterpretationResult, InterpreterError},
    query_ast::*,
    QueryResult,
};
use connector::ConnectionLike;

pub async fn execute(
    tx: &mut dyn ConnectionLike,
    write_query: WriteQuery,
    trace_id: Option<String>,
) -> InterpretationResult<QueryResult> {
    match write_query {
        WriteQuery::CreateRecord(q) => create_one(tx, q, trace_id).await,
        WriteQuery::CreateManyRecords(q) => create_many(tx, q, trace_id).await,
        WriteQuery::UpdateRecord(q) => update_one(tx, q, trace_id).await,
        WriteQuery::DeleteRecord(q) => delete_one(tx, q, trace_id).await,
        WriteQuery::UpdateManyRecords(q) => update_many(tx, q, trace_id).await,
        WriteQuery::DeleteManyRecords(q) => delete_many(tx, q, trace_id).await,
        WriteQuery::ConnectRecords(q) => connect(tx, q).await,
        WriteQuery::DisconnectRecords(q) => disconnect(tx, q, trace_id).await,
        WriteQuery::ExecuteRaw(q) => execute_raw(tx, q).await,
        WriteQuery::QueryRaw(q) => query_raw(tx, q).await,
    }
}

async fn query_raw(tx: &mut dyn ConnectionLike, q: RawQuery) -> InterpretationResult<QueryResult> {
    let res = tx.query_raw(q.model.as_ref(), q.inputs, q.query_type).await?;

    Ok(QueryResult::Json(res))
}

async fn execute_raw(tx: &mut dyn ConnectionLike, q: RawQuery) -> InterpretationResult<QueryResult> {
    let res = tx.execute_raw(q.inputs).await?;
    let num = serde_json::Value::Number(serde_json::Number::from(res));

    Ok(QueryResult::Json(num))
}

async fn create_one(
    tx: &mut dyn ConnectionLike,
    q: CreateRecord,
    trace_id: Option<String>,
) -> InterpretationResult<QueryResult> {
    let res = tx.create_record(&q.model, q.args, trace_id).await?;

    Ok(QueryResult::Id(Some(res)))
}

async fn create_many(
    tx: &mut dyn ConnectionLike,
    q: CreateManyRecords,
    trace_id: Option<String>,
) -> InterpretationResult<QueryResult> {
    let affected_records = tx.create_records(&q.model, q.args, q.skip_duplicates, trace_id).await?;

    Ok(QueryResult::Count(affected_records))
}

async fn update_one(
    tx: &mut dyn ConnectionLike,
    q: UpdateRecord,
    trace_id: Option<String>,
) -> InterpretationResult<QueryResult> {
    let mut res = tx.update_records(&q.model, q.record_filter, q.args, trace_id).await?;

    Ok(QueryResult::Id(res.pop()))
}

async fn delete_one(
    tx: &mut dyn ConnectionLike,
    q: DeleteRecord,
    trace_id: Option<String>,
) -> InterpretationResult<QueryResult> {
    // We need to ensure that we have a record finder, else we delete everything (conversion to empty filter).
    let filter = match q.record_filter {
        Some(f) => Ok(f),
        None => Err(InterpreterError::InterpretationError(
            "No record filter specified for delete record operation. Aborting.".to_owned(),
            None,
        )),
    }?;

    let res = tx.delete_records(&q.model, filter, trace_id).await?;

    Ok(QueryResult::Count(res))
}

async fn update_many(
    tx: &mut dyn ConnectionLike,
    q: UpdateManyRecords,
    trace_id: Option<String>,
) -> InterpretationResult<QueryResult> {
    let res = tx.update_records(&q.model, q.record_filter, q.args, trace_id).await?;

    Ok(QueryResult::Count(res.len()))
}

async fn delete_many(
    tx: &mut dyn ConnectionLike,
    q: DeleteManyRecords,
    trace_id: Option<String>,
) -> InterpretationResult<QueryResult> {
    let res = tx.delete_records(&q.model, q.record_filter, trace_id).await?;

    Ok(QueryResult::Count(res))
}

async fn connect(tx: &mut dyn ConnectionLike, q: ConnectRecords) -> InterpretationResult<QueryResult> {
    tx.m2m_connect(
        &q.relation_field,
        &q.parent_id.expect("Expected parent record ID to be set for connect"),
        &q.child_ids,
    )
    .await?;

    Ok(QueryResult::Unit)
}

async fn disconnect(
    tx: &mut dyn ConnectionLike,
    q: DisconnectRecords,
    trace_id: Option<String>,
) -> InterpretationResult<QueryResult> {
    tx.m2m_disconnect(
        &q.relation_field,
        &q.parent_id.expect("Expected parent record ID to be set for disconnect"),
        &q.child_ids,
        trace_id,
    )
    .await?;

    Ok(QueryResult::Unit)
}
