use crate::{interpreter::InterpretationResult, query_ast::*, result_ast::*};
use connector::{self, ConnectionLike, QueryArguments, ReadOperations};
use futures::future::{BoxFuture, FutureExt};
use prisma_models::{ManyRecords, RecordIdentifier};

pub fn execute<'a, 'b>(
    tx: &'a ConnectionLike<'a, 'b>,
    query: ReadQuery,
    parent_result: Option<&'a ManyRecords>,
) -> BoxFuture<'a, InterpretationResult<QueryResult>> {
    let fut = async move {
        match query {
            ReadQuery::RecordQuery(q) => read_one(tx, q).await,
            ReadQuery::ManyRecordsQuery(q) => read_many(tx, q).await,
            ReadQuery::RelatedRecordsQuery(q) => read_related(tx, q, parent_result).await,
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
        let model = query.model;
        let filter = query.filter.expect("Expected filter to be set for ReadOne query.");
        let scalars = tx.get_single_record(&model, &filter, &query.selected_fields).await?;
        let model_id = model.primary_identifier();

        match scalars {
            Some(record) => {
                let records: ManyRecords = record.into();
                let nested: Vec<QueryResult> = process_nested(tx, query.nested, Some(&records)).await?;

                Ok(QueryResult::RecordSelection(RecordSelection {
                    name: query.name,
                    fields: query.selection_order,
                    scalars: records,
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
        let scalars = tx
            .get_many_records(&query.model, query.args.clone(), &query.selected_fields)
            .await?;

        let model_id = query.model.primary_identifier();
        // let ids = scalars.identifiers(&model_id)?;
        let nested: Vec<QueryResult> = process_nested(tx, query.nested, Some(&scalars)).await?;

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
    parent_result: Option<&'a ManyRecords>,
) -> BoxFuture<'a, InterpretationResult<QueryResult>> {
    let fut = async move {
        // The query construction must guarantee that the parent result
        // contains the selected fields necessary to satisfy the relation query.
        // There are 2 options:
        // - The query already has IDs set - use those.
        // - The IDs need to be extracted from the parent result.
        let relation_parent_ids = match query.relation_parent_ids {
            Some(ref ids) => ids,
            None => {
                let relation_id = query.parent_field.identifier();
                parent_result.identifiers(relation_id)?
            }
        };

        let scalars = tx
            .get_related_records(
                &query.parent_field,
                relation_parent_ids,
                query.args.clone(),
                &query.selected_fields,
            )
            .await?;

        let model = query.parent_field.related_model();
        let model_id = model.identifier();
        let nested: Vec<QueryResult> = process_nested(tx, query.nested, Some(&scalars)).await?;

        Ok(QueryResult::RecordSelection(RecordSelection {
            name: query.name,
            fields: query.selection_order,
            query_arguments: query.args,
            model_id,
            scalars,
            nested,
        }))

        todo!()
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

fn process_nested<'a, 'b>(
    tx: &'a ConnectionLike<'a, 'b>,
    nested: Vec<ReadQuery>,
    parent_result: Option<&'a ManyRecords>,
) -> BoxFuture<'a, InterpretationResult<Vec<QueryResult>>> {
    let fut = async move {
        let mut results = Vec::with_capacity(nested.len());

        for query in nested {
            let result = execute(tx, query, parent_result).await?;
            results.push(result);
        }

        Ok(results)
    };

    fut.boxed()
}
