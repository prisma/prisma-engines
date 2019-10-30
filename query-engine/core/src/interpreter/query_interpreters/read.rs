use crate::{interpreter::InterpretationResult, query_ast::*, result_ast::*};
use connector::{self, QueryArguments, ScalarListValues, TransactionLike};
use prisma_models::{GraphqlId, ScalarField, SelectedFields};
use std::sync::Arc;

pub fn execute(
    tx: &mut dyn TransactionLike,
    query: ReadQuery,
    parent_ids: &[GraphqlId],
) -> InterpretationResult<QueryResult> {
    match query {
        ReadQuery::RecordQuery(q) => read_one(tx, q),
        ReadQuery::ManyRecordsQuery(q) => read_many(tx, q),
        ReadQuery::RelatedRecordsQuery(q) => read_related(tx, q, parent_ids),
        ReadQuery::AggregateRecordsQuery(q) => aggregate(tx, q),
    }
}

/// Queries a single record.
fn read_one(tx: &mut dyn TransactionLike, query: RecordQuery) -> InterpretationResult<QueryResult> {
    let selected_fields = inject_required_fields(query.selected_fields.clone());
    let scalars = tx.get_single_record(query.record_finder.as_ref().unwrap(), &selected_fields)?;

    let model = Arc::clone(&query.record_finder.unwrap().field.model());
    let id_field = model.fields().id().name.clone();

    match scalars {
        Some(record) => {
            let ids = vec![record.collect_id(&id_field)?];
            let list_fields = selected_fields.scalar_lists();
            let lists = resolve_scalar_list_fields(tx, ids.clone(), list_fields)?;
            let nested: Vec<QueryResult> = query
                .nested
                .into_iter()
                .map(|q| execute(tx, q, &ids))
                .collect::<InterpretationResult<Vec<QueryResult>>>()?;

            Ok(QueryResult::RecordSelection(RecordSelection {
                name: query.name,
                fields: query.selection_order,
                scalars: record.into(),
                nested,
                lists,
                id_field,
                ..Default::default()
            }))
        }
        None => Ok(QueryResult::RecordSelection(RecordSelection {
            name: query.name,
            fields: query.selection_order,
            id_field,
            ..Default::default()
        })),
    }
}

/// Queries a set of records.
fn read_many(tx: &mut dyn TransactionLike, query: ManyRecordsQuery) -> InterpretationResult<QueryResult> {
    let selected_fields = inject_required_fields(query.selected_fields.clone());
    let scalars = tx.get_many_records(Arc::clone(&query.model), query.args.clone(), &selected_fields)?;

    let model = Arc::clone(&query.model);
    let id_field = model.fields().id().name.clone();
    let ids = scalars.collect_ids(&id_field)?;
    let list_fields = selected_fields.scalar_lists();
    let lists = resolve_scalar_list_fields(tx, ids.clone(), list_fields)?;
    let nested: Vec<QueryResult> = query
        .nested
        .into_iter()
        .map(|q| execute(tx, q, &ids))
        .collect::<InterpretationResult<Vec<QueryResult>>>()?;

    Ok(QueryResult::RecordSelection(RecordSelection {
        name: query.name,
        fields: query.selection_order,
        query_arguments: query.args,
        scalars,
        nested,
        lists,
        id_field,
    }))
}

/// Queries related records for a set of parent IDs.
fn read_related(
    tx: &mut dyn TransactionLike,
    query: RelatedRecordsQuery,
    parent_ids: &[GraphqlId],
) -> InterpretationResult<QueryResult> {
    let selected_fields = inject_required_fields(query.selected_fields.clone());
    let parent_ids = match query.parent_ids {
        Some(ref ids) => ids,
        None => parent_ids,
    };

    let scalars = tx.get_related_records(
        Arc::clone(&query.parent_field),
        parent_ids,
        query.args.clone(),
        &selected_fields,
    )?;

    let model = Arc::clone(&query.parent_field.related_model());
    let id_field = model.fields().id().name.clone();
    let ids = scalars.collect_ids(&id_field)?;
    let list_fields = selected_fields.scalar_lists();
    let lists = resolve_scalar_list_fields(tx, ids.clone(), list_fields)?;
    let nested: Vec<QueryResult> = query
        .nested
        .into_iter()
        .map(|q| execute(tx, q, &ids))
        .collect::<InterpretationResult<Vec<QueryResult>>>()?;

    Ok(QueryResult::RecordSelection(RecordSelection {
        name: query.name,
        fields: query.selection_order,
        query_arguments: query.args,
        scalars,
        nested,
        lists,
        id_field,
    }))
}

fn aggregate(tx: &mut dyn TransactionLike, query: AggregateRecordsQuery) -> InterpretationResult<QueryResult> {
    let result = tx.count_by_model(query.model, QueryArguments::default())?;
    Ok(QueryResult::Count(result))
}

/// Resolves scalar lists for a list field for a set of parent IDs.
fn resolve_scalar_list_fields(
    tx: &mut dyn TransactionLike,
    record_ids: Vec<GraphqlId>,
    list_fields: Vec<Arc<ScalarField>>,
) -> connector::Result<Vec<(String, Vec<ScalarListValues>)>> {
    if !list_fields.is_empty() {
        list_fields
            .into_iter()
            .map(|list_field| {
                let name = list_field.name.clone();
                tx.get_scalar_list_values(list_field, record_ids.clone())
                    .map(|r| (name, r))
            })
            .collect::<connector::Result<Vec<(String, Vec<_>)>>>()
    } else {
        Ok(vec![])
    }
}

/// Injects fields required for querying, if they're not already in the selection set.
/// Currently, required fields for every query are:
/// - ID field
fn inject_required_fields(mut selected_fields: SelectedFields) -> SelectedFields {
    let id_field = selected_fields.model().fields().id();

    if selected_fields
        .scalar
        .iter()
        .find(|f| f.field.name == id_field.name)
        .is_none()
    {
        selected_fields.add_scalar(id_field);
    }

    selected_fields
}
