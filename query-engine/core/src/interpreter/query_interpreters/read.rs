use crate::{interpreter::InterpretationResult, query_ast::*, result_ast::*};
use connector::{self, ConnectionLike, QueryArguments, ReadOperations, ScalarListValues};
use futures::future::{BoxFuture, FutureExt};
use prisma_models::{GraphqlId, ScalarField, SelectedFields};
use std::sync::Arc;

pub fn execute<'a, 'b>(
    tx: &'a ConnectionLike<'a, 'b>,
    query: ReadQuery,
    parent_ids: &'a [GraphqlId],
) -> BoxFuture<'a, InterpretationResult<QueryResult>> {
    let fut = async move {
        match query {
            ReadQuery::RecordQuery(q) => read_one(tx, q).await,
            ReadQuery::ManyRecordsQuery(q) => read_many(tx, q).await,
            ReadQuery::RelatedRecordsQuery(q) => read_related(tx, q, parent_ids).await,
            ReadQuery::AggregateRecordsQuery(q) => aggregate(tx, q).await,
        }
    };

    fut.boxed()
}

/// Queries a single record.
fn read_one<'conn, 'tx>(
    tx: &'conn ConnectionLike<'conn, 'tx>,
    query: RecordQuery,
) -> BoxFuture<'conn, InterpretationResult<QueryResult>> {
    let fut = async move {

        let selected_fields = inject_required_fields(query.selected_fields.clone());
        let scalars = tx
            .get_single_record(query.record_finder.as_ref().unwrap(), &selected_fields)
            .await?;

        dbg!(&scalars);

        let model = query.record_finder.unwrap().field.model();
        let id_field = model.fields().id().name.clone();

        match scalars {
            Some(record) => {
                let ids = vec![record.collect_id(&id_field)?];
                 let nested: Vec<QueryResult> = process_nested(tx, query.nested, &ids).await?;

                Ok(QueryResult::RecordSelection(RecordSelection {
                    name: query.name,
                    fields: query.selection_order,
                    scalars: record.into(),
                    nested,
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
    };

    fut.boxed()
}

/// Queries a set of records.
fn read_many<'a, 'b>(
    tx: &'a ConnectionLike<'a, 'b>,
    query: ManyRecordsQuery,
) -> BoxFuture<'a, InterpretationResult<QueryResult>> {
    let fut = async move {
        let selected_fields = inject_required_fields(query.selected_fields.clone());
        let scalars = tx
            .get_many_records(&query.model, query.args.clone(), &selected_fields)
            .await?;

        let id_field = query.model.fields().id().name.clone();
        let ids = scalars.collect_ids(&id_field)?;
        let nested: Vec<QueryResult> = process_nested(tx, query.nested, &ids).await?;

        Ok(QueryResult::RecordSelection(RecordSelection {
            name: query.name,
            fields: query.selection_order,
            query_arguments: query.args,
            scalars,
            nested,
            id_field,
        }))
    };

    fut.boxed()
}

/// Queries related records for a set of parent IDs.
fn read_related<'a, 'b>(
    tx: &'a ConnectionLike<'a, 'b>,
    query: RelatedRecordsQuery,
    parent_ids: &'a [GraphqlId],
) -> BoxFuture<'a, InterpretationResult<QueryResult>> {
    let fut = async move {
        let selected_fields = inject_required_fields(query.selected_fields.clone());
        let parent_ids = match query.parent_ids {
            Some(ref ids) => ids,
            None => parent_ids,
        };

        let scalars = tx
            .get_related_records(&query.parent_field, parent_ids, query.args.clone(), &selected_fields)
            .await?;

        let model = query.parent_field.related_model();
        let id_field = model.fields().id().name.clone();
        let ids = scalars.collect_ids(&id_field)?;
         let nested: Vec<QueryResult> = process_nested(tx, query.nested, &ids).await?;

        Ok(QueryResult::RecordSelection(RecordSelection {
            name: query.name,
            fields: query.selection_order,
            query_arguments: query.args,
            scalars,
            nested,
            id_field,
        }))
    };

    fut.boxed()
}

async fn aggregate<'a, 'b>(
    tx: &'a ConnectionLike<'a, 'b>,
    query: AggregateRecordsQuery,
) -> InterpretationResult<QueryResult> {
    let result = tx.count_by_model(&query.model, QueryArguments::default()).await?;
    Ok(QueryResult::Count(result))
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

fn process_nested<'a, 'b>(
    tx: &'a ConnectionLike<'a, 'b>,
    nested: Vec<ReadQuery>,
    parent_ids: &'a [GraphqlId],
) -> BoxFuture<'a, InterpretationResult<Vec<QueryResult>>> {
    let fut = async move {
        let mut results = vec![];

        for query in nested {
            let result = execute(tx, query, &parent_ids).await?;
            results.push(result);
        }

        Ok(results)
    };

    fut.boxed()
}
