#[cfg(feature = "relation_joins")]
mod coerce;
#[cfg(feature = "relation_joins")]
mod process;

use crate::{QueryExt, Queryable, SqlError};

use connector_interface::*;
use futures::stream::{FuturesUnordered, StreamExt};
use quaint::ast::*;
use query_builder::QueryArgumentsExt;
use query_structure::*;
use sql_query_builder::{
    column_metadata,
    read::{self, no_alias},
    AsColumns, AsTable, Context, RelationFieldExt,
};

pub(crate) async fn get_single_record(
    conn: &dyn Queryable,
    model: &Model,
    filter: &Filter,
    selected_fields: &FieldSelection,
    relation_load_strategy: RelationLoadStrategy,
    ctx: &Context<'_>,
) -> crate::Result<Option<SingleRecord>> {
    match relation_load_strategy {
        #[cfg(feature = "relation_joins")]
        RelationLoadStrategy::Join => get_single_record_joins(conn, model, filter, selected_fields, ctx).await,
        #[cfg(not(feature = "relation_joins"))]
        RelationLoadStrategy::Join => unreachable!(),
        RelationLoadStrategy::Query => get_single_record_wo_joins(conn, model, filter, selected_fields, ctx).await,
    }
}

#[cfg(feature = "relation_joins")]
async fn get_single_record_joins(
    conn: &dyn Queryable,
    model: &Model,
    filter: &Filter,
    selected_fields: &FieldSelection,
    ctx: &Context<'_>,
) -> crate::Result<Option<SingleRecord>> {
    use coerce::coerce_record_with_json_relation;

    let selected_fields = selected_fields.to_virtuals_last();
    let field_names: Vec<_> = selected_fields.prisma_names_grouping_virtuals().collect();
    let idents = selected_fields.type_identifiers_with_arities_grouping_virtuals();

    let indexes = get_selection_indexes(
        selected_fields.relations().collect(),
        selected_fields.virtuals().collect(),
        &field_names,
    );

    let query = sql_query_builder::select::SelectBuilder::build(
        QueryArguments::from((model.clone(), filter.clone())),
        &selected_fields,
        ctx,
    );

    let mut record = execute_find_one(conn, query, &idents, &field_names, ctx).await?;

    if let Some(record) = record.as_mut() {
        coerce_record_with_json_relation(record, &indexes)?;
    };

    Ok(record.map(|record| SingleRecord { record, field_names }))
}

async fn get_single_record_wo_joins(
    conn: &dyn Queryable,
    model: &Model,
    filter: &Filter,
    selected_fields: &FieldSelection,
    ctx: &Context<'_>,
) -> crate::Result<Option<SingleRecord>> {
    let selected_fields = selected_fields.without_relations().into_virtuals_last();

    let query = read::get_records(
        model,
        ModelProjection::from(&selected_fields)
            .as_columns(ctx)
            .mark_all_selected(),
        selected_fields.virtuals(),
        filter,
        ctx,
    );

    let field_names: Vec<_> = selected_fields.db_names().collect();

    let idents = selected_fields.type_identifiers_with_arities();

    let record = execute_find_one(conn, query, &idents, &field_names, ctx)
        .await?
        .map(|record| SingleRecord { record, field_names });

    Ok(record)
}

async fn execute_find_one(
    conn: &dyn Queryable,
    query: Select<'_>,
    idents: &[(TypeIdentifier, FieldArity)],
    field_names: &[String],
    ctx: &Context<'_>,
) -> crate::Result<Option<Record>> {
    let meta = column_metadata::create(field_names, idents);

    let row = (match conn.find(query, meta.as_slice(), ctx).await {
        Ok(result) => Ok(Some(result)),
        Err(_e @ SqlError::RecordNotFoundForWhere(_)) => Ok(None),
        Err(_e @ SqlError::RecordDoesNotExist { .. }) => Ok(None),
        Err(e) => Err(e),
    })?
    .map(Record::from);

    Ok(row)
}

pub(crate) async fn get_many_records(
    conn: &dyn Queryable,
    model: &Model,
    query_arguments: QueryArguments,
    selected_fields: &FieldSelection,
    relation_load_strategy: RelationLoadStrategy,
    ctx: &Context<'_>,
) -> crate::Result<ManyRecords> {
    match relation_load_strategy {
        #[cfg(feature = "relation_joins")]
        RelationLoadStrategy::Join => get_many_records_joins(conn, model, query_arguments, selected_fields, ctx).await,
        #[cfg(not(feature = "relation_joins"))]
        RelationLoadStrategy::Join => unreachable!(),
        RelationLoadStrategy::Query => {
            get_many_records_wo_joins(conn, model, query_arguments, selected_fields, ctx).await
        }
    }
}

#[cfg(feature = "relation_joins")]
async fn get_many_records_joins(
    conn: &dyn Queryable,
    _model: &Model,
    query_arguments: QueryArguments,
    selected_fields: &FieldSelection,
    ctx: &Context<'_>,
) -> crate::Result<ManyRecords> {
    use coerce::coerce_record_with_json_relation;
    use std::borrow::Cow;

    let selected_fields = selected_fields.to_virtuals_last();
    let field_names: Vec<_> = selected_fields.prisma_names_grouping_virtuals().collect();
    let idents = selected_fields.type_identifiers_with_arities_grouping_virtuals();
    let meta = column_metadata::create(field_names.as_slice(), idents.as_slice());

    let indexes = get_selection_indexes(
        selected_fields.relations().collect(),
        selected_fields.virtuals().collect(),
        &field_names,
    );

    let mut records = ManyRecords::new(field_names.clone());

    if let Take::Some(0) = query_arguments.take {
        return Ok(records);
    };

    match ctx.max_bind_values() {
        Some(chunk_size) if query_arguments.should_batch(chunk_size) => {
            return Err(SqlError::QueryParameterLimitExceeded(
                "Joined queries cannot be split into multiple queries.".to_string(),
            ));
        }
        _ => (),
    };

    let query = sql_query_builder::select::SelectBuilder::build(query_arguments.clone(), &selected_fields, ctx);

    for item in conn.filter(query.into(), meta.as_slice(), ctx).await?.into_iter() {
        let mut record = Record::from(item);

        // Coerces json values to prisma values
        coerce_record_with_json_relation(&mut record, &indexes)?;

        records.push(record)
    }

    if query_arguments.needs_inmemory_processing_with_joins() {
        records.records = process::InMemoryProcessorForJoins::new(&query_arguments, records.records)
            .process(|record| Some((Cow::Borrowed(record), Cow::Borrowed(&records.field_names))))
            .collect();
    }

    Ok(records)
}

async fn get_many_records_wo_joins(
    conn: &dyn Queryable,
    model: &Model,
    mut query_arguments: QueryArguments,
    selected_fields: &FieldSelection,
    ctx: &Context<'_>,
) -> crate::Result<ManyRecords> {
    let selected_fields = selected_fields.without_relations().into_virtuals_last();
    let reversed = query_arguments.needs_reversed_order();

    let field_names: Vec<_> = selected_fields.db_names().collect();
    let idents = selected_fields.type_identifiers_with_arities();

    let meta = column_metadata::create(field_names.as_slice(), idents.as_slice());
    let mut records = ManyRecords::new(field_names.clone());

    if let Take::Some(0) = query_arguments.take {
        return Ok(records);
    };

    // Todo: This can't work for all cases. Cursor-based pagination will not work, because it relies on the ordering
    // to determine the right queries to fire, and will default to incorrect orderings if no ordering is found.
    // The should_batch has been adjusted to reflect that as a band-aid, but deeper investigation is necessary.
    match ctx.max_bind_values() {
        Some(chunk_size) if query_arguments.should_batch(chunk_size) => {
            if query_arguments.has_unbatchable_ordering() {
                return Err(SqlError::QueryParameterLimitExceeded(
                    "Your query cannot be split into multiple queries because of the order by aggregation or relevance"
                        .to_string(),
                ));
            }

            if query_arguments.has_unbatchable_filters() {
                return Err(SqlError::QueryParameterLimitExceeded(
                    "Parameter limits for this database provider require this query to be split into multiple queries, but the negation filters used prevent the query from being split. Please reduce the used values in the query."
                        .to_string(),
                ));
            }

            // We don't need to order in the database due to us ordering in this function.
            let order = std::mem::take(&mut query_arguments.order_by);

            let batches = query_arguments.batched(chunk_size);
            let mut futures = FuturesUnordered::new();

            for args in batches.into_iter() {
                let query = read::get_records(
                    model,
                    ModelProjection::from(&selected_fields)
                        .as_columns(ctx)
                        .mark_all_selected(),
                    selected_fields.virtuals(),
                    args,
                    ctx,
                );

                futures.push(conn.filter(query.into(), meta.as_slice(), ctx));
            }

            while let Some(result) = futures.next().await {
                for item in result?.into_iter() {
                    records.push(Record::from(item))
                }
            }

            if !order.is_empty() {
                records.order_by(&order, reversed)
            }
        }
        _ => {
            let query = read::get_records(
                model,
                ModelProjection::from(&selected_fields)
                    .as_columns(ctx)
                    .mark_all_selected(),
                selected_fields.virtuals(),
                query_arguments,
                ctx,
            );

            for item in conn.filter(query.into(), meta.as_slice(), ctx).await?.into_iter() {
                records.push(Record::from(item))
            }
        }
    }

    if reversed {
        records.reverse();
    }

    Ok(records)
}

pub(crate) async fn get_related_m2m_record_ids(
    conn: &dyn Queryable,
    from_field: &RelationFieldRef,
    from_record_ids: &[SelectionResult],
    ctx: &Context<'_>,
) -> crate::Result<Vec<(SelectionResult, SelectionResult)>> {
    let mut idents = vec![];
    idents.extend(ModelProjection::from(from_field.model().primary_identifier()).type_identifiers_with_arities());
    idents
        .extend(ModelProjection::from(from_field.related_model().primary_identifier()).type_identifiers_with_arities());

    let mut field_names = Vec::new();
    field_names.extend(from_field.model().primary_identifier().db_names());
    field_names.extend(from_field.related_model().primary_identifier().db_names());

    let meta = column_metadata::create(&field_names, &idents);

    let relation = from_field.relation();
    let table = relation.as_table(ctx);

    let from_column = from_field.related_field().m2m_column(ctx);
    let to_column = from_field.m2m_column(ctx);

    // [DTODO] To verify: We might need chunked fetch here (too many parameters in the query).
    let select = Select::from_table(table)
        .so_that(sql_query_builder::in_conditions(
            &[from_column.clone()],
            from_record_ids,
            ctx,
        ))
        .columns([from_column, to_column]);

    let parent_model_id = from_field.model().primary_identifier();
    let child_model_id = from_field.related_model().primary_identifier();

    let from_sfs: Vec<_> = parent_model_id
        .as_scalar_fields()
        .expect("Parent model ID has non-scalar fields.");

    let to_sfs: Vec<_> = child_model_id
        .as_scalar_fields()
        .expect("Child model ID has non-scalar fields.");

    // first parent id, then child id
    Ok(conn
        .filter(select.into(), meta.as_slice(), ctx)
        .await?
        .into_iter()
        .map(|row| {
            let mut values = row.values;

            let child_values = values.split_off(from_sfs.len());
            let parent_values = values;

            let p: SelectionResult = from_sfs
                .iter()
                .zip(parent_values)
                .map(|(sf, val)| (sf.clone(), val))
                .collect::<Vec<_>>()
                .into();

            let c: SelectionResult = to_sfs
                .iter()
                .zip(child_values)
                .map(|(sf, val)| (sf.clone(), val))
                .collect::<Vec<_>>()
                .into();

            (p, c)
        })
        .collect())
}

pub(crate) async fn aggregate(
    conn: &dyn Queryable,
    model: &Model,
    query_arguments: QueryArguments,
    selections: Vec<AggregationSelection>,
    group_by: Vec<ScalarFieldRef>,
    having: Option<Filter>,
    ctx: &Context<'_>,
) -> crate::Result<Vec<AggregationRow>> {
    if !group_by.is_empty() {
        group_by_aggregate(conn, model, query_arguments, selections, group_by, having, ctx).await
    } else {
        plain_aggregate(conn, model, query_arguments, selections, ctx)
            .await
            .map(|v| vec![v])
    }
}

async fn plain_aggregate(
    conn: &dyn Queryable,
    model: &Model,
    query_arguments: QueryArguments,
    selections: Vec<AggregationSelection>,
    ctx: &Context<'_>,
) -> crate::Result<Vec<AggregationResult>> {
    let query = read::aggregate(model, &selections, query_arguments, no_alias(), ctx);

    let idents: Vec<_> = selections
        .iter()
        .flat_map(|aggregator| aggregator.identifiers())
        .map(|ident| (ident.typ, ident.arity))
        .collect();

    let meta = column_metadata::create_anonymous(&idents);

    let mut rows = conn.filter(query.into(), meta.as_slice(), ctx).await?;
    let row = rows
        .pop()
        .expect("Expected exactly one return row for aggregation query.");

    Ok(row.into_aggregation_results(&selections))
}

async fn group_by_aggregate(
    conn: &dyn Queryable,
    model: &Model,
    query_arguments: QueryArguments,
    selections: Vec<AggregationSelection>,
    group_by: Vec<ScalarFieldRef>,
    having: Option<Filter>,
    ctx: &Context<'_>,
) -> crate::Result<Vec<AggregationRow>> {
    let query = read::group_by_aggregate(model, query_arguments, &selections, group_by, having, no_alias(), ctx);

    let idents: Vec<_> = selections
        .iter()
        .flat_map(|aggregator| aggregator.identifiers())
        .map(|ident| (ident.typ, ident.arity))
        .collect();

    let meta = column_metadata::create_anonymous(&idents);
    let rows = conn.filter(query.into(), meta.as_slice(), ctx).await?;

    Ok(rows
        .into_iter()
        .map(|row| row.into_aggregation_results(&selections))
        .collect())
}

/// Find the indexes of the relation records and the virtual selection objects to traverse a set of
/// records faster when coercing JSON values.
#[cfg(feature = "relation_joins")]
fn get_selection_indexes<'a>(
    relations: Vec<&'a RelationSelection>,
    virtuals: Vec<&'a VirtualSelection>,
    field_names: &'a [String],
) -> Vec<(usize, coerce::IndexedSelection<'a>)> {
    use coerce::IndexedSelection;

    field_names
        .iter()
        .enumerate()
        .filter_map(|(idx, field_name)| {
            relations
                .iter()
                .find_map(|rs| (rs.field.name() == field_name).then_some(IndexedSelection::Relation(rs)))
                .or_else(|| {
                    virtuals.iter().find_map(|vs| {
                        let obj_name = vs.serialized_name().0;
                        (obj_name == field_name).then_some(IndexedSelection::Virtual(obj_name))
                    })
                })
                .map(|indexed_selection| (idx, indexed_selection))
        })
        .collect()
}
