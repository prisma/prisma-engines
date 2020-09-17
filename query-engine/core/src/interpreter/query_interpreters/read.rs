use super::*;
use crate::{interpreter::InterpretationResult, query_ast::*, result_ast::*};
use connector::{self, ConnectionLike, QueryArguments, ReadOperations};
use futures::future::{BoxFuture, FutureExt};
use inmemory_record_processor::InMemoryRecordProcessor;
use prisma_models::ManyRecords;

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
        let model_id = model.primary_identifier();
        let filter = query.filter.expect("Expected filter to be set for ReadOne query.");
        let scalars = tx.get_single_record(&model, &filter, &query.selected_fields).await?;

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
                    query_arguments: QueryArguments::new(model),
                }))
            }

            None => Ok(QueryResult::RecordSelection(RecordSelection {
                name: query.name,
                fields: query.selection_order,
                model_id,
                scalars: ManyRecords::default(),
                nested: vec![],
                query_arguments: QueryArguments::new(model),
            })),
        }
    };

    fut.boxed()
}

/// Queries a set of records.
/// If the query specifies distinct, we need to lift up pagination (and distinct) processing to the core with in-memory record processing.
/// -> Distinct can't be processed in the DB with our current query API model.
///    We need to select IDs / uniques alongside the distincts, which doesn't work in SQL, as all records
///    are distinct by definition if a unique is in the selection set.
/// -> Unstable cursors can't reliably be fetched by the underlying datasource, so we need to process part of it in-memory.
fn read_many<'a, 'b>(
    tx: &'a ConnectionLike<'a, 'b>,
    mut query: ManyRecordsQuery,
) -> BoxFuture<'a, InterpretationResult<QueryResult>> {
    let fut = async move {
        let scalars = if query.args.distinct.is_some()
            || query.args.contains_unstable_cursor()
            || query.args.contains_null_cursor()
        {
            let processor = InMemoryRecordProcessor::new_from_query_args(&mut query.args);
            let scalars = tx
                .get_many_records(&query.model, query.args.clone(), &query.selected_fields)
                .await?;

            processor.apply(scalars)
        } else {
            tx.get_many_records(&query.model, query.args.clone(), &query.selected_fields)
                .await?
        };

        let model_id = query.model.primary_identifier();
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
    mut query: RelatedRecordsQuery,
    parent_result: Option<&'a ManyRecords>,
) -> BoxFuture<'a, InterpretationResult<QueryResult>> {
    let fut = async move {
        let relation = query.parent_field.relation();
        let is_m2m = relation.is_many_to_many();
        let processor = InMemoryRecordProcessor::new_from_query_args(&mut query.args);

        let scalars = if is_m2m {
            nested_read::m2m(tx, &query, parent_result, processor).await?
        } else {
            nested_read::one2m(
                tx,
                &query.parent_field,
                query.parent_projections,
                parent_result,
                query.args.clone(),
                &query.selected_fields,
                processor,
            )
            .await?
        };

        let model = query.parent_field.related_model();
        let model_id = model.primary_identifier();
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

async fn aggregate<'a, 'b>(
    tx: &'a ConnectionLike<'a, 'b>,
    query: AggregateRecordsQuery,
) -> InterpretationResult<QueryResult> {
    let selection_order = query.selection_order;
    let results = tx
        .aggregate_records(&query.model, query.aggregators, query.args)
        .await?;

    Ok(QueryResult::RecordAggregation(RecordAggregation {
        selection_order,
        results,
    }))
}

fn process_nested<'a, 'b>(
    tx: &'a ConnectionLike<'a, 'b>,
    nested: Vec<ReadQuery>,
    parent_result: Option<&'a ManyRecords>,
) -> BoxFuture<'a, InterpretationResult<Vec<QueryResult>>> {
    let fut = async move {
        let results = if matches!(parent_result, Some(parent_records) if parent_records.records.is_empty()) {
            //this catches most cases where there is no parent to cause a nested query. but sometimes even with parent records,
            // we do not need to do roundtrips which is why the nested_reads contain additional logic
            vec![]
        } else {
            let mut nested_results = Vec::with_capacity(nested.len());

            for query in nested {
                let result = execute(tx, query, parent_result).await?;
                nested_results.push(result);
            }

            nested_results
        };
        Ok(results)
    };

    fut.boxed()
}
