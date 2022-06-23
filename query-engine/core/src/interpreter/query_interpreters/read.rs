use super::*;
use crate::{interpreter::InterpretationResult, query_ast::*, result_ast::*};
use connector::{self, ConnectionLike, NestedRead, QueryArguments, RelAggregationRow, RelAggregationSelection};
use futures::future::{BoxFuture, FutureExt};
use inmemory_record_processor::InMemoryRecordProcessor;
use itertools::Itertools;
use prisma_models::{ManyRecords, ModelRef, PrismaValue, Record, SelectedField, SelectionResult};
use std::collections::{HashMap, HashSet};

pub fn execute<'conn>(
    tx: &'conn mut dyn ConnectionLike,
    query: ReadQuery,
    parent_result: Option<&'conn ManyRecords>,
    trace_id: Option<String>,
) -> BoxFuture<'conn, InterpretationResult<QueryResult>> {
    let fut = async move {
        match query {
            ReadQuery::RecordQuery(q) => read_one(tx, q, trace_id).await,
            ReadQuery::ManyRecordsQuery(q) => read_many(tx, q, trace_id).await,
            ReadQuery::RelatedRecordsQuery(q) => read_related(tx, q, parent_result, trace_id).await,
            ReadQuery::AggregateRecordsQuery(q) => aggregate(tx, q, trace_id).await,
        }
    };

    fut.boxed()
}

/// Queries a single record.
fn read_one(
    tx: &mut dyn ConnectionLike,
    query: RecordQuery,
    trace_id: Option<String>,
) -> BoxFuture<'_, InterpretationResult<QueryResult>> {
    let fut = async move {
        let model = query.model;
        let filter = query.filter.expect("Expected filter to be set for ReadOne query.");
        let scalars = tx
            .get_single_record(
                &model,
                &filter,
                &query.selected_fields,
                &query.aggregation_selections,
                trace_id,
            )
            .await?;

        match scalars {
            Some(record) => {
                let scalars: ManyRecords = record.into();
                let (scalars, aggregation_rows) =
                    extract_aggregation_rows_from_scalars(scalars, query.aggregation_selections);
                let nested: Vec<QueryResult> = process_nested(tx, query.nested, Some(&scalars)).await?;

                Ok(RecordSelection {
                    name: query.name,
                    fields: query.selection_order,
                    scalars,
                    nested,
                    query_arguments: QueryArguments::new(model.clone()),
                    model,
                    aggregation_rows,
                }
                .into())
            }

            None => Ok(QueryResult::RecordSelection(Box::new(RecordSelection {
                name: query.name,
                fields: query.selection_order,
                scalars: ManyRecords::default(),
                nested: vec![],
                query_arguments: QueryArguments::new(model.clone()),
                model,
                aggregation_rows: None,
            }))),
        }
    };

    fut.boxed()
}

fn to_nested_reads(nested: Vec<ReadQuery>) -> Vec<NestedRead> {
    let nested_reads = nested
        .into_iter()
        .map(|r| match r {
            ReadQuery::RelatedRecordsQuery(rrq) => NestedRead {
                name: rrq.name,
                alias: rrq.alias,
                parent_field: rrq.parent_field,
                args: rrq.args,
                selected_fields: rrq.selected_fields,
                nested: to_nested_reads(rrq.nested),
                selection_order: rrq.selection_order,
                aggregation_selections: rrq.aggregation_selections,
                parent_results: rrq.parent_results,
            },
            _ => todo!(),
        })
        .collect_vec();

    nested_reads
}

/// Queries a set of records.
/// If the query specifies distinct, we need to lift up pagination (and distinct) processing to the core with in-memory record processing.
/// -> Distinct can't be processed in the DB with our current query API model.
///    We need to select IDs / uniques alongside the distincts, which doesn't work in SQL, as all records
///    are distinct by definition if a unique is in the selection set.
/// -> Unstable cursors can't reliably be fetched by the underlying datasource, so we need to process part of it in-memory.
fn read_many(
    tx: &mut dyn ConnectionLike,
    mut query: ManyRecordsQuery,
    trace_id: Option<String>,
) -> BoxFuture<'_, InterpretationResult<QueryResult>> {
    let nested_reads = to_nested_reads(query.nested.clone());

    let fut = async move {
        let (mut scalars, aggregation_rows) = if query.args.requires_inmemory_processing() {
            let processor = InMemoryRecordProcessor::new_from_query_args(&mut query.args);
            let scalars = tx
                .get_many_records(
                    &query.model,
                    query.args.clone(),
                    &query.selected_fields,
                    &nested_reads,
                    &query.aggregation_selections,
                    trace_id,
                )
                .await?;
            let scalars = processor.apply(scalars);
            let (scalars, aggregation_rows) =
                extract_aggregation_rows_from_scalars(scalars, query.aggregation_selections);

            (scalars, aggregation_rows)
        } else {
            let scalars = tx
                .get_many_records(
                    &query.model,
                    query.args.clone(),
                    &query.selected_fields,
                    &nested_reads,
                    &query.aggregation_selections,
                    trace_id,
                )
                .await?;
            let (scalars, aggregation_rows) =
                extract_aggregation_rows_from_scalars(scalars, query.aggregation_selections);
            (scalars, aggregation_rows)
        };

        let nested = process_nested_read(&query.model, &nested_reads, &mut scalars)?;

        // let nested: Vec<QueryResult> = process_nested(tx, query.nested, Some(&scalars)).await?;

        let record_selection = RecordSelection {
            name: query.name,
            fields: query.selection_order,
            scalars,
            nested,
            query_arguments: query.args,
            model: query.model,
            aggregation_rows,
        };

        dbg!(&record_selection);

        Ok(record_selection.into())
    };

    fut.boxed()
}

/// Queries related records for a set of parent IDs.
fn read_related<'conn>(
    tx: &'conn mut dyn ConnectionLike,
    mut query: RelatedRecordsQuery,
    parent_result: Option<&'conn ManyRecords>,
    trace_id: Option<String>,
) -> BoxFuture<'conn, InterpretationResult<QueryResult>> {
    let fut = async move {
        let relation = query.parent_field.relation();
        let is_m2m = relation.is_many_to_many();
        let processor = InMemoryRecordProcessor::new_from_query_args(&mut query.args);

        let (scalars, aggregation_rows) = if is_m2m {
            nested_read::m2m(tx, &query, parent_result, processor, trace_id).await?
        } else {
            nested_read::one2m(
                tx,
                &query.parent_field,
                query.parent_results,
                parent_result,
                query.args.clone(),
                &query.selected_fields,
                query.aggregation_selections,
                processor,
                trace_id,
            )
            .await?
        };

        let model = query.parent_field.related_model();
        let nested: Vec<QueryResult> = process_nested(tx, query.nested, Some(&scalars)).await?;

        Ok(RecordSelection {
            name: query.name,
            fields: query.selection_order,
            scalars,
            nested,
            query_arguments: query.args,
            model,
            aggregation_rows,
        }
        .into())
    };

    fut.boxed()
}

async fn aggregate(
    tx: &mut dyn ConnectionLike,
    query: AggregateRecordsQuery,
    trace_id: Option<String>,
) -> InterpretationResult<QueryResult> {
    let selection_order = query.selection_order;

    let results = tx
        .aggregate_records(
            &query.model,
            query.args,
            query.selectors,
            query.group_by,
            query.having,
            trace_id,
        )
        .await?;

    Ok(QueryResult::RecordAggregations(RecordAggregations {
        selection_order,
        results,
    }))
}

fn process_nested_read(
    model: &ModelRef,
    nested: &[NestedRead],
    scalars: &mut ManyRecords,
) -> crate::Result<Vec<QueryResult>> {
    let mut query_results = vec![];

    let parent_ids = scalars.extract_selection_results(&model.primary_identifier())?;
    let parent_id = parent_ids.first().unwrap();

    for read in nested {
        let field_names: HashSet<String> = read
            .selected_fields
            .selections()
            .enumerate()
            .map(|(i, _)| read.db_alias(i))
            .collect();

        let indexes_to_remove: Vec<_> = scalars
            .field_names
            .iter()
            .enumerate()
            .filter_map(|(i, field_name)| {
                if field_names.contains(field_name) {
                    Some(i)
                } else {
                    None
                }
            })
            .collect();

        let mut nested_scalars: Vec<Vec<PrismaValue>> = vec![];
        let mut n_record_removed = 0;

        for index_to_remove in indexes_to_remove.into_iter() {
            let index_to_remove = index_to_remove - n_record_removed;

            // Remove all nested field names
            scalars.field_names.remove(index_to_remove);

            // Remove and collect all nested values
            for (r_index, record) in scalars.records.iter_mut().enumerate() {
                let val = record.values.remove(index_to_remove);

                // Group the nested values by record
                match nested_scalars.get_mut(r_index) {
                    Some(inner_vec) => inner_vec.push(val),
                    None => nested_scalars.push(vec![val]),
                }
            }

            n_record_removed += 1;
        }

        let nested_scalars = nested_scalars
            .into_iter()
            .map(|values| {
                let mut rec = Record::new(values);
                rec.set_parent_id(parent_id.clone());

                rec
            })
            .collect_vec();

        let many_record = ManyRecords {
            field_names: read.selected_fields.prisma_names().collect_vec(),
            records: nested_scalars,
        };

        let res = RecordSelection {
            name: read.name.clone(),
            model: read.parent_field.related_model(),
            aggregation_rows: None,
            fields: read.selected_fields.prisma_names().collect_vec(),
            nested: process_nested_read(&read.parent_field.related_model(), &read.nested, scalars)?,
            query_arguments: read.args.clone(),
            scalars: many_record,
        };

        query_results.push(QueryResult::RecordSelection(Box::new(res)));
    }

    Ok(query_results)
}

fn process_nested<'conn>(
    tx: &'conn mut dyn ConnectionLike,
    nested: Vec<ReadQuery>,
    parent_result: Option<&'conn ManyRecords>,
) -> BoxFuture<'conn, InterpretationResult<Vec<QueryResult>>> {
    let fut = async move {
        let results = if matches!(parent_result, Some(parent_records) if parent_records.records.is_empty()) {
            // This catches most cases where there is no parent to cause a nested query. but sometimes even with parent records,
            // we do not need to do roundtrips which is why the nested_reads contain additional logic
            vec![]
        } else {
            let mut nested_results = Vec::with_capacity(nested.len());

            for query in nested {
                let result = execute(tx, query, parent_result, None).await?;
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
/// This means the SQL result we get back from the database contains additional aggregation data that needs to be remapped according to the schema
/// This function takes care of removing the aggregation data from the database result and collects it separately
/// so that it can be serialized separately later according to the schema
pub fn extract_aggregation_rows_from_scalars(
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
