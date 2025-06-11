use crate::{
    interpreter::{InterpretationResult, InterpreterError},
    query_ast::*,
    QueryResult, RecordSelection,
};
use connector::{ConnectionLike, NativeUpsert};
use query_structure::{ManyRecords, RawJson};
use sql_query_builder::write::split_write_args_by_shape;
use telemetry::TraceParent;

pub(crate) async fn execute(
    tx: &mut dyn ConnectionLike,
    write_query: WriteQuery,
    traceparent: Option<TraceParent>,
) -> InterpretationResult<QueryResult> {
    match write_query {
        WriteQuery::CreateRecord(q) => create_one(tx, q, traceparent).await,
        WriteQuery::CreateManyRecords(q) => create_many(tx, q, traceparent).await,
        WriteQuery::UpdateRecord(q) => update_one(tx, q, traceparent).await,
        WriteQuery::DeleteRecord(q) => delete_one(tx, q, traceparent).await,
        WriteQuery::UpdateManyRecords(q) => update_many(tx, q, traceparent).await,
        WriteQuery::DeleteManyRecords(q) => delete_many(tx, q, traceparent).await,
        WriteQuery::ConnectRecords(q) => connect(tx, q, traceparent).await,
        WriteQuery::DisconnectRecords(q) => disconnect(tx, q, traceparent).await,
        WriteQuery::ExecuteRaw(q) => execute_raw(tx, q).await,
        WriteQuery::QueryRaw(q) => query_raw(tx, q).await,
        WriteQuery::Upsert(q) => native_upsert(tx, q, traceparent).await,
    }
}

async fn query_raw(tx: &mut dyn ConnectionLike, q: RawQuery) -> InterpretationResult<QueryResult> {
    let res = tx.query_raw(q.model.as_ref(), q.inputs, q.query_type).await?;

    Ok(QueryResult::RawJson(res))
}

async fn execute_raw(tx: &mut dyn ConnectionLike, q: RawQuery) -> InterpretationResult<QueryResult> {
    let res = tx.execute_raw(q.inputs).await?;
    let num = serde_json::Value::Number(serde_json::Number::from(res));

    Ok(QueryResult::RawJson(
        RawJson::try_new(num).map_err(|err| InterpreterError::Generic(err.to_string()))?,
    ))
}

async fn create_one(
    tx: &mut dyn ConnectionLike,
    q: CreateRecord,
    traceparent: Option<TraceParent>,
) -> InterpretationResult<QueryResult> {
    let res = tx
        .create_record(&q.model, q.args, q.selected_fields, traceparent)
        .await?;

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
    traceparent: Option<TraceParent>,
) -> InterpretationResult<QueryResult> {
    if q.split_by_shape {
        return create_many_split_by_shape(tx, q, traceparent).await;
    }

    if let Some(selected_fields) = q.selected_fields {
        let records = tx
            .create_records_returning(&q.model, q.args, q.skip_duplicates, selected_fields.fields, traceparent)
            .await?;

        let nested: Vec<QueryResult> =
            super::read::process_nested(tx, selected_fields.nested, Some(&records), traceparent).await?;

        let selection = RecordSelection {
            name: q.name,
            fields: selected_fields.order,
            records,
            nested,
            model: q.model,
            virtual_fields: vec![],
        };

        Ok(QueryResult::RecordSelection(Some(Box::new(selection))))
    } else {
        let affected_records = tx
            .create_records(&q.model, q.args, q.skip_duplicates, traceparent)
            .await?;

        Ok(QueryResult::Count(affected_records))
    }
}

/// Performs bulk inserts grouped by record shape.
///
/// This is required to support connectors which do not support `DEFAULT` in the list of values for `INSERT`.
/// See [`create_many_shape`] for more information as to which heuristic we use to group create many entries.
async fn create_many_split_by_shape(
    tx: &mut dyn ConnectionLike,
    q: CreateManyRecords,
    traceparent: Option<TraceParent>,
) -> InterpretationResult<QueryResult> {
    if let Some(selected_fields) = q.selected_fields {
        let mut result: Option<ManyRecords> = None;

        for args in split_write_args_by_shape(&q.model, q.args) {
            let current_batch = tx
                .create_records_returning(
                    &q.model,
                    args,
                    q.skip_duplicates,
                    selected_fields.fields.clone(),
                    traceparent,
                )
                .await?;

            if let Some(result) = &mut result {
                // We assume that all records have the same set and order of fields,
                // since we pass the same `selected_fields.fields` to the
                // `create_records_returning()` above.
                result.records.extend(current_batch.records.into_iter());
            } else {
                result = Some(current_batch);
            }
        }

        let records = if let Some(result) = result {
            result
        } else {
            // Empty result means that the list of arguments was empty as well.
            tx.create_records_returning(&q.model, vec![], q.skip_duplicates, selected_fields.fields, traceparent)
                .await?
        };

        let nested: Vec<QueryResult> =
            super::read::process_nested(tx, selected_fields.nested.clone(), Some(&records), traceparent).await?;

        let selection = RecordSelection {
            name: q.name,
            fields: selected_fields.order,
            records,
            nested,
            model: q.model,
            virtual_fields: vec![],
        };

        Ok(QueryResult::RecordSelection(Some(Box::new(selection))))
    } else {
        let mut result = 0;

        for args in split_write_args_by_shape(&q.model, q.args) {
            let affected_records = tx
                .create_records(&q.model, args, q.skip_duplicates, traceparent)
                .await?;
            result += affected_records;
        }

        Ok(QueryResult::Count(result))
    }
}

async fn update_one(
    tx: &mut dyn ConnectionLike,
    q: UpdateRecord,
    traceparent: Option<TraceParent>,
) -> InterpretationResult<QueryResult> {
    let res = tx
        .update_record(
            q.model(),
            q.record_filter().clone(),
            q.args().clone(),
            q.selected_fields(),
            traceparent,
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
                .map(|record| record.extract_selection_result(&q.model().shard_aware_primary_identifier()))
                .transpose()?;

            Ok(QueryResult::Id(res))
        }
    }
}

async fn native_upsert(
    tx: &mut dyn ConnectionLike,
    query: NativeUpsert,
    traceparent: Option<TraceParent>,
) -> InterpretationResult<QueryResult> {
    let scalars = tx.native_upsert_record(query.clone(), traceparent).await?;

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
    traceparent: Option<TraceParent>,
) -> InterpretationResult<QueryResult> {
    // We need to ensure that we have a record finder, else we delete everything (conversion to empty filter).
    let filter = q.record_filter;

    if let Some(selected_fields) = q.selected_fields {
        let record = tx
            .delete_record(&q.model, filter, selected_fields.fields, traceparent)
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
        let result = tx.delete_records(&q.model, filter, None, traceparent).await?;
        Ok(QueryResult::Count(result))
    }
}

async fn update_many(
    tx: &mut dyn ConnectionLike,
    q: UpdateManyRecords,
    traceparent: Option<TraceParent>,
) -> InterpretationResult<QueryResult> {
    if let Some(selected_fields) = q.selected_fields {
        let records = tx
            .update_records_returning(
                &q.model,
                q.record_filter,
                q.args,
                selected_fields.fields,
                q.limit,
                traceparent,
            )
            .await?;

        let nested: Vec<QueryResult> =
            super::read::process_nested(tx, selected_fields.nested, Some(&records), traceparent).await?;

        let selection = RecordSelection {
            name: q.name,
            fields: selected_fields.order,
            records,
            nested,
            model: q.model,
            virtual_fields: vec![],
        };

        Ok(QueryResult::RecordSelection(Some(Box::new(selection))))
    } else {
        let affected_records = tx
            .update_records(&q.model, q.record_filter, q.args, q.limit, traceparent)
            .await?;

        Ok(QueryResult::Count(affected_records))
    }
}

async fn delete_many(
    tx: &mut dyn ConnectionLike,
    q: DeleteManyRecords,
    traceparent: Option<TraceParent>,
) -> InterpretationResult<QueryResult> {
    let res = tx
        .delete_records(&q.model, q.record_filter, q.limit, traceparent)
        .await?;

    Ok(QueryResult::Count(res))
}

async fn connect(
    tx: &mut dyn ConnectionLike,
    q: ConnectRecords,
    traceparent: Option<TraceParent>,
) -> InterpretationResult<QueryResult> {
    tx.m2m_connect(
        &q.relation_field,
        &q.parent_id.expect("Expected parent record ID to be set for connect"),
        &q.child_ids,
        traceparent,
    )
    .await?;

    Ok(QueryResult::Unit)
}

async fn disconnect(
    tx: &mut dyn ConnectionLike,
    q: DisconnectRecords,
    traceparent: Option<TraceParent>,
) -> InterpretationResult<QueryResult> {
    tx.m2m_disconnect(
        &q.relation_field,
        &q.parent_id.expect("Expected parent record ID to be set for disconnect"),
        &q.child_ids,
        traceparent,
    )
    .await?;

    Ok(QueryResult::Unit)
}
