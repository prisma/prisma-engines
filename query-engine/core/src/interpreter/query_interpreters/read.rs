use crate::{interpreter::InterpretationResult, query_ast::*, result_ast::*};
use connector::{self, ConnectionLike, QueryArguments, ReadOperations};
use futures::future::{BoxFuture, FutureExt};
use prisma_models::{Field, RecordIdentifier, SelectedFields};

pub fn execute<'a, 'b>(
    tx: &'a ConnectionLike<'a, 'b>,
    query: ReadQuery,
    parent_ids: &'a [RecordIdentifier],
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
        let model = query.model;
        let filter = query.filter.expect("Expected filter to be set for ReadOne query.");
        let scalars = tx.get_single_record(&model, &filter, &selected_fields).await?;
        let model_id = model.identifier();

        match scalars {
            Some(record) => {
                let ids = vec![record.identifier(&model_id)?];
                let nested: Vec<QueryResult> = process_nested(tx, query.nested, &ids).await?;

                Ok(QueryResult::RecordSelection(RecordSelection {
                    name: query.name,
                    fields: query.selection_order,
                    scalars: record.into(),
                    nested,
                    model_id,
                    ..Default::default()
                }))
            }

            None => Ok(QueryResult::RecordSelection(RecordSelection {
                name: query.name,
                fields: query.selection_order,
                model_id,
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

        let model_id = query.model.identifier();
        let ids = scalars.identifiers(&model_id)?;
        let nested: Vec<QueryResult> = process_nested(tx, query.nested, &ids).await?;

        Ok(QueryResult::RecordSelection(RecordSelection {
            name: query.name,
            fields: query.selection_order,
            query_arguments: query.args,
            model_id,
            scalars,
            nested,
        }))
    };

    fut.boxed()
}

/// Queries related records for a set of parent IDs.
fn read_related<'a, 'b>(
    tx: &'a ConnectionLike<'a, 'b>,
    query: RelatedRecordsQuery,
    parent_ids: &'a [RecordIdentifier],
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
        let model_id = model.identifier();
        let ids = scalars.identifiers(&model_id)?;
        let nested: Vec<QueryResult> = process_nested(tx, query.nested, &ids).await?;

        Ok(QueryResult::RecordSelection(RecordSelection {
            name: query.name,
            fields: query.selection_order,
            query_arguments: query.args,
            model_id,
            scalars,
            nested,
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

/// Injects fields required for querying data, if they're not already in the selection set.
/// Currently, required fields for every query are the fields of the model identifier.
fn inject_required_fields(mut selected_fields: SelectedFields) -> SelectedFields {
    let model_id = selected_fields.model().identifier();
    let selected_names = selected_fields.names();

    let missing_fields: Vec<Field> = model_id
        .into_iter()
        .filter(|field| selected_names.find(|name| name == &field.name()).is_none())
        .collect();

    for field in missing_fields {
        selected_fields.add(field)
    }

    selected_fields
}

fn process_nested<'a, 'b>(
    tx: &'a ConnectionLike<'a, 'b>,
    nested: Vec<ReadQuery>,
    parent_ids: &'a [RecordIdentifier],
) -> BoxFuture<'a, InterpretationResult<Vec<QueryResult>>> {
    let fut = async move {
        let mut results = Vec::with_capacity(nested.len());

        for query in nested {
            let result = execute(tx, query, &parent_ids).await?;
            results.push(result);
        }

        Ok(results)
    };

    fut.boxed()
}
