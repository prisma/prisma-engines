use crate::{interpreter::InterpretationResult, query_ast::*, result_ast::*};
use connector::{self, filter::Filter, ConnectionLike, QueryArguments, ReadOperations, ScalarCompare};
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

        let other_fields: Vec<_> = query
            .parent_field
            .related_field()
            .linking_fields()
            .fields()
            .flat_map(|f| f.data_source_fields())
            .collect();

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

        let mut scalars = tx
            .get_many_records(&query.parent_field.related_model(), args, &query.selected_fields)
            .await?;

        dbg!(&scalars);

        // Write parent IDs into the retrieved records
        if parent_result.is_some() && query.parent_field.is_inlined_in_enclosing_model() {
            let parent_identifier = query.parent_field.model().primary_identifier();
            let field_names = scalars.field_names.clone();

            let parent_link_fields = query.parent_field.linking_fields();
            let child_link_fields = query.parent_field.related_field().linking_fields();

            let parent_result =
                parent_result.expect("No parent results present in the query graph for reading related records.");

            let parent_fields = &parent_result.field_names;
            let mut additional_records = vec![];

            for mut record in scalars.records.iter_mut() {
                let child_link: RecordIdentifier = record.identifier(&field_names, &child_link_fields)?;

                let mut parent_records = parent_result.records.iter().filter(|record| {
                    let parent_link = record.identifier(parent_fields, &parent_link_fields).unwrap();

                    child_link.values().eq(parent_link.values())
                });

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
        } else if parent_result.is_some() && query.parent_field.related_field().is_inlined_in_enclosing_model() {
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
        }

        dbg!(&scalars);

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
