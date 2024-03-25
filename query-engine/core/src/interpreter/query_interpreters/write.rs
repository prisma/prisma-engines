use std::collections::HashMap;

use crate::{
    interpreter::{InterpretationResult, InterpreterError},
    query_ast::*,
    QueryResult, RecordSelection,
};
use connector::{ConnectionLike, DatasourceFieldName, NativeUpsert, WriteArgs};
use query_structure::ManyRecords;

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
    if q.split_by_shape {
        return create_many_split_by_shape(tx, q, trace_id).await;
    }

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

/// Performs bulk inserts grouped by record shape.
///
/// By "record shape" we mean "unique set of fields". This is required to support connectors which
/// do not support `DEFAULT` in the list of values for `INSERT`. For these, we need to rely on the
/// fact that the database will use the default value for the column if it is not listed in `INSERT`
/// statement. Grouping by record shape allows us to use this property, because each batch of fields
/// has the same columns listed for the `INSERT` statement.
async fn create_many_split_by_shape(
    tx: &mut dyn ConnectionLike,
    q: CreateManyRecords,
    trace_id: Option<String>,
) -> InterpretationResult<QueryResult> {
    let mut args_by_shape: HashMap<Vec<DatasourceFieldName>, Vec<WriteArgs>> = Default::default();
    for write_args in q.args {
        let mut shape: Vec<_> = write_args.args.keys().cloned().collect();
        // This ensures that shapes is not dependent on order of fields.
        shape.sort_unstable();
        args_by_shape.entry(shape).or_default().push(write_args);
    }

    if let Some(selected_fields) = q.selected_fields {
        let mut result: Option<ManyRecords> = None;
        for args in args_by_shape.into_values() {
            let current_batch = tx
                .create_records_returning(
                    &q.model,
                    args,
                    q.skip_duplicates,
                    selected_fields.fields.clone(),
                    trace_id.clone(),
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
            tx.create_records_returning(&q.model, vec![], q.skip_duplicates, selected_fields.fields, trace_id)
                .await?
        };

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
        let mut result = 0;
        for args in args_by_shape.into_values() {
            let affected_records = tx
                .create_records(&q.model, args, q.skip_duplicates, trace_id.clone())
                .await?;
            result += affected_records;
        }
        Ok(QueryResult::Count(result))
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
