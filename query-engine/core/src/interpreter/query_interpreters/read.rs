use super::*;
use crate::{interpreter::InterpretationResult, query_ast::*, result_ast::*};
use connector::{self, ConnectionLike, QueryArguments, ReadOperations, RelAggregationRow, RelAggregationSelection};
use futures::future::{BoxFuture, FutureExt};
use inmemory_record_processor::InMemoryRecordProcessor;
use prisma_models::ManyRecords;
use std::collections::HashMap;

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
        let scalars = tx
            .get_single_record(&model, &filter, &query.selected_fields, &query.aggregation_selections)
            .await?;

        match scalars {
            Some(record) => {
                let scalars: ManyRecords = record.into();
                let (scalars, aggregation_rows) =
                    extract_aggregation_rows_from_scalars(scalars.clone(), query.aggregation_selections);
                let nested: Vec<QueryResult> = process_nested(tx, query.nested, Some(&scalars)).await?;

                Ok(RecordSelection {
                    name: query.name,
                    fields: query.selection_order,
                    scalars,
                    nested,
                    model_id,
                    query_arguments: QueryArguments::new(model),
                    aggregation_rows,
                }
                .into())
            }

            None => Ok(QueryResult::RecordSelection(Box::new(RecordSelection {
                name: query.name,
                fields: query.selection_order,
                model_id,
                scalars: ManyRecords::default(),
                nested: vec![],
                query_arguments: QueryArguments::new(model),
                aggregation_rows: None,
            }))),
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
        let (scalars, aggregation_rows) = if query.args.requires_inmemory_processing() {
            let processor = InMemoryRecordProcessor::new_from_query_args(&mut query.args);
            let scalars = tx
                .get_many_records(
                    &query.model,
                    query.args.clone(),
                    &query.selected_fields,
                    &query.aggregation_selections,
                )
                .await?;
            let (scalars, aggregation_rows) =
                extract_aggregation_rows_from_scalars(scalars.clone(), query.aggregation_selections);

            (processor.apply(scalars), aggregation_rows)
        } else {
            let scalars = tx
                .get_many_records(
                    &query.model,
                    query.args.clone(),
                    &query.selected_fields,
                    &query.aggregation_selections,
                )
                .await?;
            let (scalars, aggregation_rows) =
                extract_aggregation_rows_from_scalars(scalars.clone(), query.aggregation_selections);

            (scalars, aggregation_rows)
        };

        let model_id = query.model.primary_identifier();
        let nested: Vec<QueryResult> = process_nested(tx, query.nested, Some(&scalars)).await?;

        Ok(RecordSelection {
            name: query.name,
            fields: query.selection_order,
            query_arguments: query.args,
            model_id,
            scalars,
            nested,
            aggregation_rows,
        }
        .into())
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

        Ok(RecordSelection {
            name: query.name,
            fields: query.selection_order,
            query_arguments: query.args,
            model_id,
            scalars,
            nested,
            aggregation_rows: None,
        }
        .into())
    };

    fut.boxed()
}

async fn aggregate<'a, 'b>(
    tx: &'a ConnectionLike<'a, 'b>,
    query: AggregateRecordsQuery,
) -> InterpretationResult<QueryResult> {
    let selection_order = query.selection_order;

    let results = tx
        .aggregate_records(&query.model, query.args, query.selectors, query.group_by, query.having)
        .await?;

    Ok(QueryResult::RecordAggregations(RecordAggregations {
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

/// Removes the relation aggregation data from the database result and collect it into some RelAggregationRow
/// Explanation: Relation aggregations on a findMany are selected from an output object type. eg:
/// findManyX { _count { rel_1, rel 2 } }
/// Output object types are typically used for selecting relations, so they're are queried in a different request
/// In the case of relation aggregations though, we query that data along side the request sent for the base model ("X" in the query above)
/// This means the SQL result we get back from the database contains additional aggregation data that needs to be remapped according to the shema
/// This function takes care of removing the aggregation data from the database result and collects it separately
/// so that it can be serialized separately later according to the schema
fn extract_aggregation_rows_from_scalars(
    mut scalars: ManyRecords,
    aggr_selections: Vec<RelAggregationSelection>,
) -> (ManyRecords, Option<Vec<RelAggregationRow>>) {
    if aggr_selections.is_empty() {
        return (scalars, None);
    }

    let aggr_field_names: HashMap<String, &RelAggregationSelection> = aggr_selections
        .iter()
        .map(|aggr_sel| (aggr_sel.db_alias(), aggr_sel))
        .collect();

    let indexes_to_remove: Vec<_> = scalars
        .field_names
        .iter()
        .enumerate()
        .filter_map(|(i, field_name)| aggr_field_names.get(field_name).map(|aggr_sel| (i, *aggr_sel)))
        .collect();

    let mut aggregation_rows: Vec<RelAggregationRow> = vec![];
    let mut n_record_removed = 0;

    for (index_to_remove, aggr_sel) in indexes_to_remove.into_iter() {
        let index_to_remove = index_to_remove - n_record_removed;

        // Remove all aggr field names
        scalars.field_names.remove(index_to_remove);

        // Remove and collect all aggr prisma values
        for (r_index, record) in scalars.records.iter_mut().enumerate() {
            let val = record.values.remove(index_to_remove);
            let aggr_result = aggr_sel.clone().into_result(val);

            // Group the aggregation results by record
            match aggregation_rows.get_mut(r_index) {
                Some(inner_vec) => inner_vec.push(aggr_result),
                None => aggregation_rows.push(vec![aggr_result]),
            }
        }
        n_record_removed += 1;
    }

    (scalars, Some(aggregation_rows))
}
