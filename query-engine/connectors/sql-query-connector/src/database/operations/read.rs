use super::coerce::coerce_record_with_join;
use crate::{
    column_metadata,
    model_extensions::*,
    query_arguments_ext::QueryArgumentsExt,
    query_builder::{self, read},
    Context, QueryExt, Queryable, SqlError,
};

use connector_interface::*;
use futures::stream::{FuturesUnordered, StreamExt};
use quaint::ast::*;
use query_structure::*;

pub(crate) async fn get_single_record(
    conn: &dyn Queryable,
    model: &Model,
    filter: &Filter,
    selected_fields: &ModelProjection,
    aggr_selections: &[RelAggregationSelection],
    ctx: &Context<'_>,
) -> crate::Result<Option<SingleRecord>> {
    let query = read::get_records(
        model,
        selected_fields.as_columns(ctx).mark_all_selected(),
        aggr_selections,
        filter,
        ctx,
    );

    let mut field_names: Vec<_> = selected_fields.db_names().collect();
    let mut aggr_field_names: Vec<_> = aggr_selections.iter().map(|aggr_sel| aggr_sel.db_alias()).collect();

    field_names.append(&mut aggr_field_names);

    let mut idents = selected_fields.type_identifiers_with_arities();
    let mut aggr_idents = aggr_selections
        .iter()
        .map(|aggr_sel| aggr_sel.type_identifier_with_arity())
        .collect();

    idents.append(&mut aggr_idents);

    let meta = column_metadata::create(field_names.as_slice(), idents.as_slice());

    let record = (match conn.find(query, meta.as_slice(), ctx).await {
        Ok(result) => Ok(Some(result)),
        Err(_e @ SqlError::RecordNotFoundForWhere(_)) => Ok(None),
        Err(_e @ SqlError::RecordDoesNotExist) => Ok(None),
        Err(e) => Err(e),
    })?
    .map(Record::from)
    .map(|record| SingleRecord { record, field_names });

    Ok(record)
}

pub(crate) async fn get_many_records_joins(
    conn: &dyn Queryable,
    _model: &Model,
    query_arguments: QueryArguments,
    selected_fields: &ModelProjection,
    nested: Vec<RelatedQuery>,
    _aggr_selections: &[RelAggregationSelection],
    ctx: &Context<'_>,
) -> crate::Result<ManyRecords> {
    let mut field_names: Vec<_> = selected_fields.db_names().collect();
    field_names.extend(nested.iter().map(|n| n.parent_field.name().to_owned()));

    let mut idents = selected_fields.type_identifiers_with_arities();
    idents.extend(nested.iter().map(|_| (TypeIdentifier::Json, FieldArity::Required)));

    let meta = column_metadata::create(field_names.as_slice(), idents.as_slice());
    let rq_indexes = related_queries_indexes(&nested, field_names.as_slice());

    let mut records = ManyRecords::new(field_names.clone());

    let query = query_builder::select::build(query_arguments.clone(), nested.clone(), selected_fields, &[], ctx);

    for item in conn.filter(query.into(), meta.as_slice(), ctx).await?.into_iter() {
        let mut record = Record::from(item);

        // Coerces json values to prisma values
        coerce_record_with_join(&mut record, rq_indexes.clone());

        records.push(record)
    }

    Ok(records)
}

// TODO: find better name
fn related_queries_indexes<'a>(
    related_queries: &'a [RelatedQuery],
    field_names: &[String],
) -> Vec<(usize, &'a RelatedQuery)> {
    let mut output: Vec<(usize, &RelatedQuery)> = Vec::new();

    for (idx, field_name) in field_names.iter().enumerate() {
        if let Some(rq) = related_queries.iter().find(|rq| rq.name == *field_name) {
            output.push((idx, rq));
        }
    }

    output
}

pub(crate) async fn get_many_records(
    conn: &dyn Queryable,
    model: &Model,
    mut query_arguments: QueryArguments,
    selected_fields: &ModelProjection,
    nested: Vec<RelatedQuery>,
    aggr_selections: &[RelAggregationSelection],
    ctx: &Context<'_>,
) -> crate::Result<ManyRecords> {
    let reversed = query_arguments.needs_reversed_order();

    let mut field_names: Vec<_> = selected_fields.db_names().collect();
    let mut aggr_field_names: Vec<_> = aggr_selections.iter().map(|aggr_sel| aggr_sel.db_alias()).collect();

    field_names.append(&mut aggr_field_names);

    let mut aggr_idents = aggr_selections
        .iter()
        .map(|aggr_sel| aggr_sel.type_identifier_with_arity())
        .collect();

    let mut idents = selected_fields.type_identifiers_with_arities();

    idents.append(&mut aggr_idents);

    let meta = column_metadata::create(field_names.as_slice(), idents.as_slice());
    let mut records = ManyRecords::new(field_names.clone());

    if let Some(0) = query_arguments.take {
        return Ok(records);
    };

    // Todo: This can't work for all cases. Cursor-based pagination will not work, because it relies on the ordering
    // to determine the right queries to fire, and will default to incorrect orderings if no ordering is found.
    // The should_batch has been adjusted to reflect that as a band-aid, but deeper investigation is necessary.
    match ctx.max_bind_values {
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
                    selected_fields.as_columns(ctx).mark_all_selected(),
                    aggr_selections,
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
                selected_fields.as_columns(ctx).mark_all_selected(),
                aggr_selections,
                query_arguments,
                nested,
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

    let from_columns: Vec<_> = from_field.related_field().m2m_columns(ctx);
    let to_columns: Vec<_> = from_field.m2m_columns(ctx);

    // [DTODO] To verify: We might need chunked fetch here (too many parameters in the query).
    let select = Select::from_table(table)
        .so_that(query_builder::in_conditions(&from_columns, from_record_ids, ctx))
        .columns(from_columns.into_iter().chain(to_columns.into_iter()));

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
    let query = read::aggregate(model, &selections, query_arguments, ctx);

    let idents: Vec<_> = selections
        .iter()
        .flat_map(|aggregator| aggregator.identifiers())
        .map(|(_, ident, arity)| (ident, arity))
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
    let query = read::group_by_aggregate(model, query_arguments, &selections, group_by, having, ctx);

    let idents: Vec<_> = selections
        .iter()
        .flat_map(|aggregator| aggregator.identifiers())
        .map(|(_, ident, arity)| (ident, arity))
        .collect();

    let meta = column_metadata::create_anonymous(&idents);
    let rows = conn.filter(query.into(), meta.as_slice(), ctx).await?;

    Ok(rows
        .into_iter()
        .map(|row| row.into_aggregation_results(&selections))
        .collect())
}
