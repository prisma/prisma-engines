use crate::{
    interpreter::InterpretationResult, query_ast::*, query_graph_builder::write::utils::IdFilter, result_ast::*,
};
use connector::{self, filter::Filter, ConnectionLike, QueryArguments, ReadOperations, ScalarCompare};
use futures::future::{BoxFuture, FutureExt};
use prisma_models::{ManyRecords, OrderBy, Record, RecordIdentifier};
use prisma_value::PrismaValue;
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
        let filter = query.filter.expect("Expected filter to be set for ReadOne query.");
        let scalars = tx
            .get_single_record(&model, &filter, &query.selected_fields.only_scalar_and_inlined())
            .await?;
        let model_id = model.primary_identifier();

        match scalars {
            Some(record) => {
                let records: ManyRecords = record.into();
                dbg!(&records);
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
            .get_many_records(
                &query.model,
                query.args.clone(),
                &query.selected_fields.only_scalar_and_inlined(),
            )
            .await?;

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
    query: RelatedRecordsQuery,
    parent_result: Option<&'a ManyRecords>,
) -> BoxFuture<'a, InterpretationResult<QueryResult>> {
    let fut = async move {
        // The query construction must guarantee that the parent result
        // contains the selected fields necessary to satisfy the relation query.
        // There are 2 options:
        // - The query already has IDs set - use those.
        // - The IDs need to be extracted from the parent result.
        let is_with_pagination = query.args.is_with_pagination();
        // todo: find better approach for first
        let needs_reversing = query.args.last.is_some();
        let (skip, take) = (
            query.args.skip.unwrap_or(0),
            query.args.first.or(query.args.last).unwrap_or(999999),
        );
        let relation_parent_ids = match query.relation_parent_ids {
            Some(ids) => ids,
            None => {
                let relation_id = query.parent_field.linking_fields();
                parent_result
                    .expect("[ID retrieval] No parent results present in the query graph for reading related records.")
                    .identifiers(&relation_id)?
            }
        };

        let relation = query.parent_field.relation();

        println!("123 {:?}", &parent_result);
        println!("124 {:?}", parent_result.is_some());
        println!("125 {:?}", is_with_pagination);

        // prisma level join does not work for many 2 many yet
        // can only work if we have a parent result. This is not the case when we e.g. have nested delete inside an update
        //        let use_prisma_level_join = parent_result.is_some() && is_with_pagination();
        let use_prisma_level_join = parent_result.is_some();

        let mut scalars = if !use_prisma_level_join {
            println!("Using old code path");
            tx.get_related_records(
                &query.parent_field,
                &relation_parent_ids,
                query.args.clone(),
                &query.selected_fields.only_scalar_and_inlined(),
            )
            .await?
        } else if relation.is_many_to_many() {
            println!("141 Using new many to many code path");
            let ids = tx
                .get_related_m2m_record_ids(&query.parent_field, &relation_parent_ids)
                .await?;

            println!("146 {:?}", &ids);

            let child_model_id = query.parent_field.related_model().primary_identifier();
            let child_ids: Vec<RecordIdentifier> = ids
                .iter()
                .map(|ri| child_model_id.assimilate(ri.1.clone()))
                .collect::<std::result::Result<Vec<_>, _>>()?;

            let filter = child_ids.filter();
            let mut args = query.args.clone();

            args.filter = match args.filter {
                Some(existing_filter) => Some(Filter::and(vec![existing_filter, filter])),
                None => Some(filter),
            };

            let mut scalars = tx
                .get_many_records(
                    &query.parent_field.related_model(),
                    args,
                    &query.selected_fields.only_scalar_and_inlined(),
                )
                .await?;

            // Child id to parent ids
            let mut id_map: HashMap<RecordIdentifier, Vec<RecordIdentifier>> = HashMap::new();

            for (parent_id, child_id) in ids {
                match id_map.get_mut(&child_id) {
                    Some(v) => v.push(parent_id),
                    None => {
                        id_map.insert(child_id, vec![parent_id]);
                    }
                };
            }

            dbg!(&id_map);

            let fields = &scalars.field_names;
            let mut additional_records = vec![];

            for record in scalars.records.iter_mut() {
                let record_id = record.identifier(fields, &child_model_id)?;
                dbg!(&record_id);

                let mut parent_ids = id_map.remove(&record_id).expect("1");
                let first = parent_ids.pop().expect("2");

                record.parent_id = Some(first);

                for parent_id in parent_ids {
                    let mut record = record.clone();

                    record.parent_id = Some(parent_id);
                    additional_records.push(record);
                }
            }

            scalars.records.extend(additional_records);
            scalars
        } else {
            println!("Using new in-memory join code path");
            // PRISMA LEVEL JOIN

            let other_fields: Vec<_> = query
                .parent_field
                .related_field()
                .linking_fields()
                .data_source_fields()
                .collect();

            let is_compound_case = other_fields.len() > 1;

            let args = if is_compound_case {
                let filters: Vec<Filter> = relation_parent_ids
                    .clone()
                    .into_iter()
                    .map(|id| {
                        let filters = id
                            .pairs
                            .into_iter()
                            .zip(other_fields.iter())
                            .map(|((_, value), other_field)| other_field.equals(value))
                            .collect();
                        Filter::and(filters)
                    })
                    .collect();

                let filter = Filter::or(filters);
                let mut args = query.args.clone();
                // we do pagination ourselves and not in the db
                args.first = None;
                args.skip = None;
                args.last = None;

                args.filter = match args.filter {
                    Some(existing_filter) => Some(Filter::and(vec![existing_filter, filter])),
                    None => Some(filter),
                };
                //                args.first = Some(-1); // superb hack to force implicit ordering but not change the result of the query'
                args
            } else {
                // SINGULAR CASE
                let other_field = other_fields.first().unwrap();
                let parent_ids_as_prisma_values = relation_parent_ids.iter().map(|ri| ri.single_value()).collect();
                let filter = other_field.is_in(parent_ids_as_prisma_values);
                let mut args = query.args.clone();
                // we do pagination ourselves and not in the db
                args.first = None;
                args.skip = None;
                args.last = None;

                args.filter = match args.filter {
                    Some(existing_filter) => Some(Filter::and(vec![existing_filter, filter])),
                    None => Some(filter),
                };
                //                args.first = Some(-1); // superb hack to force implicit ordering but not change the result of the query
                args
            };

            tx.get_many_records(&query.parent_field.related_model(), args, &query.selected_fields)
                .await?
        };

        if use_prisma_level_join {
            // Write parent IDs into the retrieved records
            if parent_result.is_some() && query.parent_field.is_inlined_on_enclosing_model() {
                println!("1");
                let parent_identifier = query.parent_field.model().primary_identifier();
                let field_names = scalars.field_names.clone();

                let parent_link_fields = query.parent_field.linking_fields();
                let child_link_fields = query.parent_field.related_field().linking_fields();

                let parent_result = parent_result.expect(
                    "[Result Construction] No parent results present in the query graph for reading related records.",
                );

                let parent_fields = &parent_result.field_names;
                let mut additional_records = vec![];

                let mut records_by_parent_id: HashMap<Vec<&PrismaValue>, Vec<&Record>> = HashMap::new();
                for record in parent_result.records.iter() {
                    let prisma_values = record.identifying_values(parent_fields, &parent_link_fields).unwrap();
                    match records_by_parent_id.get_mut(&prisma_values) {
                        Some(records) => records.push(record),
                        None => {
                            let mut records = Vec::new();
                            records.push(record);
                            records_by_parent_id.insert(prisma_values, records);
                        }
                    }
                }

                for mut record in scalars.records.iter_mut() {
                    let child_link: RecordIdentifier = record.identifier(&field_names, &child_link_fields)?;

                    let child_values: Vec<&PrismaValue> = child_link.pairs.iter().map(|p| &p.1).collect();
                    let empty_vec = Vec::new();
                    let mut parent_records = records_by_parent_id.get(&child_values).unwrap_or(&empty_vec).iter();

                    let parent_id = parent_records
                        .next()
                        .unwrap()
                        .identifier(parent_fields, &parent_identifier)
                        .unwrap();

                    record.parent_id = Some(parent_id);

                    for p_record in parent_records {
                        let parent_id = p_record.identifier(parent_fields, &parent_identifier).unwrap();
                        let mut record = record.clone();

                        record.parent_id = Some(parent_id);
                        additional_records.push(record);
                    }
                }

                scalars.records.extend(additional_records);
            } else if parent_result.is_some() && query.parent_field.related_field().is_inlined_on_enclosing_model() {
                println!("2");
                let parent_identifier = query.parent_field.model().primary_identifier();
                let field_names = scalars.field_names.clone();
                let child_link_fields = query.parent_field.related_field().linking_fields();

                for record in scalars.records.iter_mut() {
                    let parent_id: RecordIdentifier = record.identifier(&field_names, &child_link_fields)?;
                    let parent_id = parent_id
                        .into_iter()
                        .zip(parent_identifier.data_source_fields())
                        .map(|((_, value), field)| (field, value))
                        .collect::<Vec<_>>()
                        .into();

                    record.parent_id = Some(parent_id);
                }

                if is_with_pagination {
                    println!("in memory pagination");
                    let mut count_by_parent_id: HashMap<Option<RecordIdentifier>, i64> = HashMap::new();
                    // replacement for SQL order by
                    println!("before sorting: {:?}", scalars.records);
                    scalars.records.sort_by_key(|r| {
                        let values: Vec<_> = r.parent_id.as_ref().unwrap().values().collect();
                        values
                    });
                    println!("after sorting: {:?}", scalars.records);
                    // apply pagination
                    if needs_reversing {
                        scalars.records.reverse();
                    }
                    scalars.records.retain(|record| {
                        let current_count = count_by_parent_id.get(&record.parent_id).unwrap_or(&0);
                        let new_count = current_count + 1;
                        count_by_parent_id.insert(record.parent_id.clone(), new_count);

                        println!("new_count: {:?}, take: {:?}, skip: {:?}", new_count, take, skip);
                        new_count > skip && new_count <= take + skip
                    });
                    println!("{:?}", &count_by_parent_id);
                } else {
                    println!("no in memory pagination");
                }
            } else if query.parent_field.relation().is_many_to_many() {
                println!("3");
            // nothing to do for many to many.
            } else {
                panic!(format!(
                    "parent result: {:?}, relation: {:?}",
                    &parent_result,
                    &query.parent_field.relation()
                ));
            }
        }

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
