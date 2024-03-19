use crate::{
    interpreter::{InterpretationResult, InterpreterError},
    query_ast::*,
    QueryResult, RecordSelection,
};
use connector::{ConnectionLike, NativeUpsert};

pub(crate) async fn execute(
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
        WriteQuery::ConnectRecords(q) => connect(tx, q, trace_id).await,
        WriteQuery::DisconnectRecords(q) => disconnect(tx, q, trace_id).await,
        WriteQuery::ExecuteRaw(q) => execute_raw(tx, q).await,
        WriteQuery::QueryRaw(q) => query_raw(tx, q).await,
        WriteQuery::Upsert(q) => native_upsert(tx, q, trace_id).await,
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
    let res = tx.create_record(&q.model, q.args, q.selected_fields, trace_id).await?;

    Ok(QueryResult::RecordSelection(Some(Box::new(RecordSelection {
        name: q.name,
        fields: q.selection_order,
        model: q.model,
        records: res.into(),
        nested: vec![],
        virtual_fields: vec![],
    }))))
}

async fn create_many(
    tx: &mut dyn ConnectionLike,
    q: CreateManyRecords,
    trace_id: Option<String>,
) -> InterpretationResult<QueryResult> {
    if let Some(selected_fields) = q.selected_fields {
        let records = tx
            .create_records_returning(&q.model, q.args, q.skip_duplicates, selected_fields.fields, trace_id)
            .await?;

        let selection = RecordSelection {
            name: q.name,
            fields: selected_fields.order,
            records,
            nested: vec![],
            model: q.model,
            virtual_fields: vec![],
        };

        Ok(QueryResult::RecordSelection(Some(Box::new(selection))))
    } else {
        let affected_records = tx.create_records(&q.model, q.args, q.skip_duplicates, trace_id).await?;
        Ok(QueryResult::Count(affected_records))
    }
}

async fn update_one(
    tx: &mut dyn ConnectionLike,
    q: UpdateRecord,
    trace_id: Option<String>,
) -> InterpretationResult<QueryResult> {
    let res = tx
        .update_record(
            q.model(),
            q.record_filter().clone(),
            q.args().clone(),
            q.selected_fields(),
            trace_id,
        )
        .await?;

    match q {
        UpdateRecord::WithSelection(q) => {
            let res = res
                .map(|res| RecordSelection {
                    name: q.name,
                    fields: q.selection_order,
                    records: res.into(),
                    nested: vec![],
                    model: q.model,
                    virtual_fields: vec![],
                })
                .map(Box::new);

            Ok(QueryResult::RecordSelection(res))
        }
        UpdateRecord::WithoutSelection(_) => {
            let res = res
                .map(|record| record.extract_selection_result(&q.model().primary_identifier()))
                .transpose()?;

            Ok(QueryResult::Id(res))
        }
    }
}

async fn native_upsert(
    tx: &mut dyn ConnectionLike,
    query: NativeUpsert,
    trace_id: Option<String>,
) -> InterpretationResult<QueryResult> {
    let scalars = tx.native_upsert_record(query.clone(), trace_id).await?;

    Ok(RecordSelection {
        name: query.name().to_string(),
        fields: query.selection_order().to_owned(),
        records: scalars.into(),
        nested: Vec::new(),
        model: query.model().clone(),
        virtual_fields: vec![],
    }
    .into())
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

    if let Some(selected_fields) = q.selected_fields {
        let record = tx
            .delete_record(&q.model, filter, selected_fields.fields, trace_id)
            .await?;
        let selection = RecordSelection {
            name: q.name,
            fields: selected_fields.order,
            records: record.into(),
            nested: vec![],
            model: q.model,
            virtual_fields: vec![],
        };

        Ok(QueryResult::RecordSelection(Some(Box::new(selection))))
    } else {
        let result = tx.delete_records(&q.model, filter, trace_id).await?;
        Ok(QueryResult::Count(result))
    }
}

async fn update_many(
    tx: &mut dyn ConnectionLike,
    q: UpdateManyRecords,
    trace_id: Option<String>,
) -> InterpretationResult<QueryResult> {
    let res = tx.update_records(&q.model, q.record_filter, q.args, trace_id).await?;

    Ok(QueryResult::Count(res))
}

async fn delete_many(
    tx: &mut dyn ConnectionLike,
    q: DeleteManyRecords,
    trace_id: Option<String>,
) -> InterpretationResult<QueryResult> {
    let res = tx.delete_records(&q.model, q.record_filter, trace_id).await?;

    Ok(QueryResult::Count(res))
}

async fn connect(
    tx: &mut dyn ConnectionLike,
    q: ConnectRecords,
    trace_id: Option<String>,
) -> InterpretationResult<QueryResult> {
    tx.m2m_connect(
        &q.relation_field,
        &q.parent_id.expect("Expected parent record ID to be set for connect"),
        &q.child_ids,
        trace_id,
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
