use crate::{
    column_metadata,
    model_extensions::*,
    query_arguments_ext::QueryArgumentsExt,
    query_builder::{self, read},
    sql_info::SqlInfo,
    QueryExt, SqlError,
};
use connector_interface::*;
use futures::stream::{FuturesUnordered, StreamExt};
use prisma_models::*;
use quaint::ast::*;

pub async fn get_single_record(
    conn: &dyn QueryExt,
    model: &ModelRef,
    filter: &Filter,
    selected_fields: &ModelProjection,
    aggr_selections: &[RelAggregationSelection],
    trace_id: Option<String>,
) -> crate::Result<Option<SingleRecord>> {
    let query = read::get_records(
        model,
        selected_fields.as_columns(),
        aggr_selections,
        filter,
        &[],
        trace_id.clone(),
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

    let record = (match conn.find(query, meta.as_slice(), trace_id).await {
        Ok(result) => Ok(Some(result)),
        Err(_e @ SqlError::RecordNotFoundForWhere(_)) => Ok(None),
        Err(_e @ SqlError::RecordDoesNotExist) => Ok(None),
        Err(e) => Err(e),
    })?
    .map(Record::from)
    .map(|record| SingleRecord { record, field_names });

    Ok(record)
}

pub async fn get_many_records(
    conn: &dyn QueryExt,
    model: &ModelRef,
    mut query_arguments: QueryArguments,
    selected_fields: &ModelProjection,
    nested_reads: &[NestedRead],
    aggr_selections: &[RelAggregationSelection],
    sql_info: SqlInfo,
    trace_id: Option<String>,
) -> crate::Result<ManyRecords> {
    let reversed = query_arguments.needs_reversed_order();

    let mut field_names: Vec<_> = selected_fields.db_names().collect();
    let mut aggr_field_names: Vec<_> = aggr_selections.iter().map(|aggr_sel| aggr_sel.db_alias()).collect();
    let mut nested_read_field_names = NestedRead::db_aliases(nested_reads, 0);

    field_names.append(&mut aggr_field_names);
    field_names.append(&mut nested_read_field_names);

    let mut idents = selected_fields.type_identifiers_with_arities();
    let mut aggr_idents = aggr_selections
        .iter()
        .map(|aggr_sel| aggr_sel.type_identifier_with_arity())
        .collect();
    let mut nested_read_idents: Vec<(_, _)> = nested_reads
        .iter()
        .flat_map(|read| read.type_identifier_with_arities())
        .collect();

    idents.append(&mut aggr_idents);
    idents.append(&mut nested_read_idents);

    let meta = column_metadata::create(field_names.as_slice(), idents.as_slice());
    let mut records = ManyRecords::new(field_names.clone());

    if let Some(0) = query_arguments.take {
        return Ok(records);
    };

    // Todo: This can't work for all cases. Cursor-based pagination will not work, because it relies on the ordering
    // to determine the right queries to fire, and will default to incorrect orderings if no ordering is found.
    // The should_batch has been adjusted to reflect that as a band-aid, but deeper investigation is necessary.
    match sql_info.max_bind_values {
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
                    selected_fields.as_columns(),
                    aggr_selections,
                    args,
                    nested_reads,
                    trace_id.clone(),
                );

                futures.push(conn.filter(query.into(), meta.as_slice(), trace_id.clone()));
            }

            while let Some(result) = futures.next().await {
                for item in result?.into_iter() {
                    records.push(Record::from(item))
                }
            }

            if !order.is_empty() {
                records.order_by(&order)
            }
        }
        _ => {
            let query = read::get_records(
                model,
                selected_fields.as_columns(),
                aggr_selections,
                query_arguments,
                nested_reads,
                trace_id.clone(),
            );

            for item in conn.filter(query.into(), meta.as_slice(), trace_id).await?.into_iter() {
                records.push(Record::from(item))
            }
        }
    }

    if reversed {
        records.reverse();
    }

    Ok(records)
}

pub async fn get_related_m2m_record_ids(
    conn: &dyn QueryExt,
    from_field: &RelationFieldRef,
    from_record_ids: &[SelectionResult],
    trace_id: Option<String>,
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
    let table = relation.as_table();

    let from_columns: Vec<_> = from_field.related_field().m2m_columns();
    let to_columns: Vec<_> = from_field.m2m_columns();

    // [DTODO] To verify: We might need chunked fetch here (too many parameters in the query).
    let select = Select::from_table(table)
        .so_that(query_builder::conditions(&from_columns, from_record_ids))
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
        .filter(select.into(), meta.as_slice(), trace_id)
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

pub async fn aggregate(
    conn: &dyn QueryExt,
    model: &ModelRef,
    query_arguments: QueryArguments,
    selections: Vec<AggregationSelection>,
    group_by: Vec<ScalarFieldRef>,
    having: Option<Filter>,
    trace_id: Option<String>,
) -> crate::Result<Vec<AggregationRow>> {
    if !group_by.is_empty() {
        group_by_aggregate(conn, model, query_arguments, selections, group_by, having, trace_id).await
    } else {
        plain_aggregate(conn, model, query_arguments, selections, trace_id)
            .await
            .map(|v| vec![v])
    }
}

async fn plain_aggregate(
    conn: &dyn QueryExt,
    model: &ModelRef,
    query_arguments: QueryArguments,
    selections: Vec<AggregationSelection>,
    trace_id: Option<String>,
) -> crate::Result<Vec<AggregationResult>> {
    let query = read::aggregate(model, &selections, query_arguments, trace_id.clone());

    let idents: Vec<_> = selections
        .iter()
        .flat_map(|aggregator| aggregator.identifiers())
        .map(|(_, ident, arity)| (ident, arity))
        .collect();

    let meta = column_metadata::create_anonymous(&idents);

    let mut rows = conn.filter(query.into(), meta.as_slice(), trace_id).await?;
    let row = rows
        .pop()
        .expect("Expected exactly one return row for aggregation query.");

    Ok(row.into_aggregation_results(&selections))
}

async fn group_by_aggregate(
    conn: &dyn QueryExt,
    model: &ModelRef,
    query_arguments: QueryArguments,
    selections: Vec<AggregationSelection>,
    group_by: Vec<ScalarFieldRef>,
    having: Option<Filter>,
    trace_id: Option<String>,
) -> crate::Result<Vec<AggregationRow>> {
    let query = read::group_by_aggregate(model, query_arguments, &selections, group_by, having, trace_id.clone());

    let idents: Vec<_> = selections
        .iter()
        .flat_map(|aggregator| aggregator.identifiers())
        .map(|(_, ident, arity)| (ident, arity))
        .collect();

    let meta = column_metadata::create_anonymous(&idents);
    let rows = conn.filter(query.into(), meta.as_slice(), trace_id).await?;

    Ok(rows
        .into_iter()
        .map(|row| row.into_aggregation_results(&selections))
        .collect())
}
