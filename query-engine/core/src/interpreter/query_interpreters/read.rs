use crate::{interpreter::InterpretationResult, query_ast::*, result_ast::*};
use connector::{self, filter::Filter, ConnectionLike, QueryArguments, ReadOperations, ScalarCompare};
use futures::future::{BoxFuture, FutureExt};
use prisma_models::{ManyRecords, Record, RecordIdentifier};
use std::collections::HashMap;
use std::time::Instant;

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
        // contains the selected fields necessary to satisfy the relation query ("relation IDs").
        // There are 2 options:
        // - The query already has IDs set - use those.
        // - The IDs need to be extracted from the parent result.
        let relation_parent_ids = match query.relation_parent_ids {
            Some(ids) => ids,
            None => {
                let relation_id = query.parent_field.linking_fields();
                parent_result
                    .expect("No parent results present in the query graph for reading related records.")
                    .identifiers(&relation_id)?
            }
        };

        let relation = query.parent_field.relation();

        // prisma level join does not work for many 2 many yet
        // can only work if we have a parent result. This is not the case when we e.g. have nested delete inside an update
        let use_prisma_level_join =
            !relation.is_many_to_many() && parent_result.is_some() && !query.args.is_with_pagination();

        let mut scalars = if !use_prisma_level_join {
            tx.get_related_records(
                &query.parent_field,
                &relation_parent_ids,
                query.args.clone(),
                &query.selected_fields.only_scalar_and_inlined(),
            )
            .await?
        } else {
            // PRISMA LEVEL JOIN

            let other_fields: Vec<_> = query
                .parent_field
                .related_field()
                .linking_fields()
                .fields()
                .flat_map(|f| f.data_source_fields())
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

                args.filter = match args.filter {
                    Some(existing_filter) => Some(Filter::and(vec![existing_filter, filter])),
                    None => Some(filter),
                };
                args
            } else {
                // SINGULAR CASE
                let other_field = other_fields.first().unwrap();
                let parent_ids_as_prisma_values = relation_parent_ids.iter().map(|ri| ri.single_value()).collect();
                let filter = other_field.is_in(parent_ids_as_prisma_values);
                let mut args = query.args.clone();

                args.filter = match args.filter {
                    Some(existing_filter) => Some(Filter::and(vec![existing_filter, filter])),
                    None => Some(filter),
                };
                args
            };

            let now = Instant::now();
            let result = tx
                .get_many_records(&query.parent_field.related_model(), args, &query.selected_fields)
                .await?;
            println!("QUERY TIME {}", now.elapsed().as_millis());
            result
        };

        let now = Instant::now();
        if use_prisma_level_join {
            // Write parent IDs into the retrieved records
            if parent_result.is_some() && query.parent_field.is_inlined_on_enclosing_model() {
                println!("JOIN ALGO 1");
                let parent_identifier = query.parent_field.model().primary_identifier();
                let field_names = &scalars.field_names;

                let parent_link_fields = query.parent_field.linking_fields();
                let child_link_fields = query.parent_field.related_field().linking_fields();

                let parent_result =
                    parent_result.expect("No parent results present in the query graph for reading related records.");

                let parent_fields = &parent_result.field_names;

                // The child (scalars) records that are linked to more than one parent will have
                // copies of themselves with the right parent ids pushed in this Vec.
                let mut additional_records = Vec::new();

                // Map from parent record identifier for the child records to parent records.
                //
                // We use raw bytes for the identifier values because we want to avoid copying
                // PrismaValues (allocations),
                let mut parent_records_index: HashMap<Vec<u8>, Vec<&Record>> = HashMap::new();

                let mut identifiers_buf = Vec::with_capacity(16);

                // Populate the identifiers index map.
                for record in parent_result.records.iter() {
                    record.identifier_bytes(parent_fields, &parent_link_fields, &mut identifiers_buf)?;

                    match parent_records_index.get_mut(&identifiers_buf) {
                        Some(records) => records.push(record),
                        None => {
                            let records = vec![record];
                            let buf_len = identifiers_buf.len();
                            let id_bytes = std::mem::replace(&mut identifiers_buf, Vec::with_capacity(buf_len));
                            parent_records_index.insert(id_bytes, records);
                        }
                    }
                }

                // Link each child record to its parents.
                for mut record in scalars.records.iter_mut() {
                    record.identifier_bytes(&field_names, &child_link_fields, &mut identifiers_buf)?;

                    let parent_records = parent_records_index.get(&identifiers_buf);
                    let mut parent_records = parent_records.iter().flat_map(|records| records.into_iter());

                    // Set the parent id on the first record so avoid copying for the first parent.
                    {
                        let parent_id = parent_records
                            .next()
                            .unwrap()
                            .identifier(parent_fields, &parent_identifier)
                            .unwrap();

                        record.parent_id = Some(parent_id);
                    }

                    // Set the parent id on every subsequent record (i.e. join a copy of the record
                    // to every parent that should be connected).
                    for p_record in parent_records {
                        let parent_id = p_record.identifier(parent_fields, &parent_identifier).unwrap();
                        let mut record = record.clone();

                        record.parent_id = Some(parent_id);
                        additional_records.push(record);
                    }
                }

                scalars.records.extend(additional_records);
            } else if parent_result.is_some() && query.parent_field.related_field().is_inlined_on_enclosing_model() {
                println!("JOIN ALGO 2");
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
            } else if query.parent_field.relation().is_many_to_many() {
                // nothing to do for many to many. parent ids are already present
            } else {
                panic!(format!(
                    "parent result: {:?}, relation: {:?}",
                    &parent_result,
                    &query.parent_field.relation()
                ));
            }
        }
        println!("JOIN TIME {}", now.elapsed().as_millis());

        let model = query.parent_field.related_model();
        let model_id = model.primary_identifier();
        let now = Instant::now();
        let nested: Vec<QueryResult> = process_nested(tx, query.nested, Some(&scalars)).await?;
        println!("NESTED TIME {}", now.elapsed().as_millis());

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
