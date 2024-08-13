use super::{inmemory_record_processor::InMemoryRecordProcessor, *};
use crate::{interpreter::InterpretationResult, query_ast::*, result_ast::*};
use connector::{error::ConnectorError, ConnectionLike};
use futures::future::{BoxFuture, FutureExt};
use psl::can_support_relation_load_strategy;
use query_structure::{ManyRecords, RelationLoadStrategy, RelationSelection};
use user_facing_errors::KnownError;

pub(crate) fn execute<'conn>(
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
        let record = tx
            .get_single_record(
                &model,
                &filter,
                &query.selected_fields,
                query.relation_load_strategy,
                trace_id,
            )
            .await?;

        match record {
            Some(record) if query.relation_load_strategy.is_query() => {
                let records = record.into();
                let nested = process_nested(tx, query.nested, Some(&records)).await?;

                Ok(RecordSelection {
                    name: query.name,
                    fields: query.selection_order,
                    records,
                    nested,
                    model,
                    virtual_fields: query.selected_fields.virtuals_owned(),
                }
                .into())
            }
            Some(record) => {
                let records: ManyRecords = record.into();

                Ok(RecordSelectionWithRelations {
                    name: query.name,
                    model,
                    fields: query.selection_order,
                    virtuals: query.selected_fields.virtuals_owned(),
                    records,
                    nested: build_relation_record_selection(query.selected_fields.relations()),
                }
                .into())
            }

            None if query.options.contains(QueryOption::ThrowOnEmpty) => record_not_found(),

            None => Ok(QueryResult::RecordSelection(Some(Box::new(RecordSelection {
                name: query.name,
                fields: query.selection_order,
                records: ManyRecords::default(),
                nested: vec![],
                model,
                virtual_fields: query.selected_fields.virtuals_owned(),
            })))),
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
fn read_many(
    tx: &mut dyn ConnectionLike,
    query: ManyRecordsQuery,
    trace_id: Option<String>,
) -> BoxFuture<'_, InterpretationResult<QueryResult>> {
    match query.relation_load_strategy {
        RelationLoadStrategy::Join => read_many_by_joins(tx, query, trace_id),
        RelationLoadStrategy::Query => read_many_by_queries(tx, query, trace_id),
    }
}

fn read_many_by_queries(
    tx: &mut dyn ConnectionLike,
    mut query: ManyRecordsQuery,
    trace_id: Option<String>,
) -> BoxFuture<'_, InterpretationResult<QueryResult>> {
    let processor = if query.args.requires_inmemory_processing() {
        Some(InMemoryRecordProcessor::new_from_query_args(&mut query.args))
    } else {
        None
    };

    let fut = async move {
        let records = tx
            .get_many_records(
                &query.model,
                query.args.clone(),
                &query.selected_fields,
                query.relation_load_strategy,
                trace_id,
            )
            .await?;

        let records = if let Some(p) = processor {
            p.apply(records)
        } else {
            records
        };

        if records.records.is_empty() && query.options.contains(QueryOption::ThrowOnEmpty) {
            record_not_found()
        } else {
            let nested: Vec<QueryResult> = process_nested(tx, query.nested, Some(&records)).await?;

            Ok(RecordSelection {
                name: query.name,
                fields: query.selection_order,
                records,
                nested,
                model: query.model,
                virtual_fields: query.selected_fields.virtuals_owned(),
            }
            .into())
        }
    };

    fut.boxed()
}

fn read_many_by_joins(
    tx: &mut dyn ConnectionLike,
    query: ManyRecordsQuery,
    trace_id: Option<String>,
) -> BoxFuture<'_, InterpretationResult<QueryResult>> {
    if !can_support_relation_load_strategy() {
        unreachable!()
    }
    let fut = async move {
        let result = tx
            .get_many_records(
                &query.model,
                query.args.clone(),
                &query.selected_fields,
                query.relation_load_strategy,
                trace_id,
            )
            .await?;

        if result.records.is_empty() && query.options.contains(QueryOption::ThrowOnEmpty) {
            record_not_found()
        } else {
            Ok(RecordSelectionWithRelations {
                name: query.name,
                fields: query.selection_order,
                virtuals: query.selected_fields.virtuals_owned(),
                records: result,
                nested: build_relation_record_selection(query.selected_fields.relations()),
                model: query.model,
            }
            .into())
        }
    };

    fut.boxed()
}

fn build_relation_record_selection<'a>(
    selections: impl Iterator<Item = &'a RelationSelection>,
) -> Vec<RelationRecordSelection> {
    selections
        .map(|rq| RelationRecordSelection {
            name: rq.field.name().to_owned(),
            fields: rq.result_fields.clone(),
            virtuals: rq.virtuals().cloned().collect(),
            model: rq.field.related_model(),
            nested: build_relation_record_selection(rq.relations()),
        })
        .collect()
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

        let records = if relation.is_many_to_many() {
            nested_read::m2m(tx, &mut query, parent_result, trace_id).await?
        } else {
            nested_read::one2m(
                tx,
                &query.parent_field,
                query.parent_results,
                parent_result,
                query.args.clone(),
                &query.selected_fields,
                trace_id,
            )
            .await?
        };
        let model = query.parent_field.related_model();
        let nested: Vec<QueryResult> = process_nested(tx, query.nested, Some(&records)).await?;

        Ok(RecordSelection {
            name: query.name,
            fields: query.selection_order,
            records,
            nested,
            model,
            virtual_fields: query.selected_fields.virtuals_owned(),
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

pub(crate) fn process_nested<'conn>(
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

// Custom error built for findXOrThrow queries, when a record is not found and it needs to throw an error
#[inline]
fn record_not_found() -> InterpretationResult<QueryResult> {
    let cause = String::from("Expected a record, found none.");
    Err(ConnectorError {
        user_facing_error: Some(KnownError::new(
            user_facing_errors::query_engine::RecordRequiredButNotFound { cause: cause.clone() },
        )),
        kind: connector::error::ErrorKind::RecordDoesNotExist { cause },
        transient: false,
    }
    .into())
}
