use crate::{
    column_metadata,
    query_arguments_ext::QueryArgumentsExt,
    query_builder::{self, read},
    QueryExt, SqlError,
};
use connector_interface::*;
use futures::stream::{FuturesUnordered, StreamExt};
use prisma_models::*;
use quaint::ast::*;

#[tracing::instrument(skip(conn, model, filter, selected_fields))]
pub async fn get_single_record(
    conn: &dyn QueryExt,
    model: &ModelRef,
    filter: &Filter,
    selected_fields: &ModelProjection,
) -> crate::Result<Option<SingleRecord>> {
    let query = read::get_records(&model, selected_fields.as_columns(), &[], filter);

    let field_names: Vec<_> = selected_fields.db_names().collect();
    let idents = selected_fields.type_identifiers_with_arities();
    let meta = column_metadata::create(field_names.as_slice(), idents.as_slice());

    let record = (match conn.find(query, meta.as_slice()).await {
        Ok(result) => Ok(Some(result)),
        Err(_e @ SqlError::RecordNotFoundForWhere(_)) => Ok(None),
        Err(_e @ SqlError::RecordDoesNotExist) => Ok(None),
        Err(e) => Err(e),
    })?
    .map(Record::from)
    .map(|record| SingleRecord { record, field_names });

    Ok(record)
}

#[tracing::instrument(skip(conn, model, query_arguments, selected_fields))]
pub async fn get_many_records(
    conn: &dyn QueryExt,
    model: &ModelRef,
    mut query_arguments: QueryArguments,
    selected_fields: &ModelProjection,
    aggr_selections: &[RelAggregationSelection],
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
    if query_arguments.should_batch() {
        // We don't need to order in the database due to us ordering in this function.
        let order = std::mem::replace(&mut query_arguments.order_by, vec![]);

        let batches = query_arguments.batched();
        let mut futures = FuturesUnordered::new();

        for args in batches.into_iter() {
            let query = read::get_records(model, selected_fields.as_columns(), aggr_selections, args);

            futures.push(conn.filter(query.into(), meta.as_slice()));
        }

        while let Some(result) = futures.next().await {
            for item in result?.into_iter() {
                records.push(Record::from(item))
            }
        }

        if !order.is_empty() {
            records.order_by(&order)
        }
    } else {
        let query = read::get_records(model, selected_fields.as_columns(), aggr_selections, query_arguments);

        for item in conn.filter(query.into(), meta.as_slice()).await?.into_iter() {
            records.push(Record::from(item))
        }
    };

    if reversed {
        records.reverse();
    }

    Ok(records)
}

#[tracing::instrument(skip(conn, from_field, from_record_ids))]
pub async fn get_related_m2m_record_ids(
    conn: &dyn QueryExt,
    from_field: &RelationFieldRef,
    from_record_ids: &[RecordProjection],
) -> crate::Result<Vec<(RecordProjection, RecordProjection)>> {
    let mut idents = vec![];
    idents.extend(from_field.model().primary_identifier().type_identifiers_with_arities());
    idents.extend(
        from_field
            .related_model()
            .primary_identifier()
            .type_identifiers_with_arities(),
    );

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

    let from_sfs: Vec<_> = parent_model_id.scalar_fields().collect();
    let to_sfs: Vec<_> = child_model_id.scalar_fields().collect();

    // first parent id, then child id
    Ok(conn
        .filter(select.into(), meta.as_slice())
        .await?
        .into_iter()
        .map(|row| {
            let mut values = row.values;

            let child_values = values.split_off(from_sfs.len());
            let parent_values = values;

            let p: RecordProjection = from_sfs
                .iter()
                .zip(parent_values)
                .map(|(sf, val)| (sf.clone(), val))
                .collect::<Vec<_>>()
                .into();

            let c: RecordProjection = to_sfs
                .iter()
                .zip(child_values)
                .map(|(sf, val)| (sf.clone(), val))
                .collect::<Vec<_>>()
                .into();

            (p, c)
        })
        .collect())
}

#[tracing::instrument(skip(conn, model, query_arguments, selections, group_by, having))]
pub async fn aggregate(
    conn: &dyn QueryExt,
    model: &ModelRef,
    query_arguments: QueryArguments,
    selections: Vec<AggregationSelection>,
    group_by: Vec<ScalarFieldRef>,
    having: Option<Filter>,
) -> crate::Result<Vec<AggregationRow>> {
    if !group_by.is_empty() {
        group_by_aggregate(conn, model, query_arguments, selections, group_by, having).await
    } else {
        plain_aggregate(conn, model, query_arguments, selections)
            .await
            .map(|v| vec![v])
    }
}

#[tracing::instrument(skip(conn, model, query_arguments, selections))]
async fn plain_aggregate(
    conn: &dyn QueryExt,
    model: &ModelRef,
    query_arguments: QueryArguments,
    selections: Vec<AggregationSelection>,
) -> crate::Result<Vec<AggregationResult>> {
    let query = read::aggregate(model, &selections, query_arguments);

    let idents: Vec<_> = selections
        .iter()
        .flat_map(|aggregator| aggregator.identifiers())
        .collect();

    let meta = column_metadata::create_anonymous(&idents);

    let mut rows = conn.filter(query.into(), meta.as_slice()).await?;
    let row = rows
        .pop()
        .expect("Expected exactly one return row for aggregation query.");

    Ok(row.into_aggregation_results(&selections))
}

#[tracing::instrument(skip(conn, model, query_arguments, selections, group_by, having))]
async fn group_by_aggregate(
    conn: &dyn QueryExt,
    model: &ModelRef,
    query_arguments: QueryArguments,
    selections: Vec<AggregationSelection>,
    group_by: Vec<ScalarFieldRef>,
    having: Option<Filter>,
) -> crate::Result<Vec<AggregationRow>> {
    let query = read::group_by_aggregate(model, query_arguments, &selections, group_by, having);

    let idents: Vec<_> = selections
        .iter()
        .flat_map(|aggregator| aggregator.identifiers())
        .collect();

    let meta = column_metadata::create_anonymous(&idents);
    let rows = conn.filter(query.into(), meta.as_slice()).await?;

    Ok(rows
        .into_iter()
        .map(|row| row.into_aggregation_results(&selections))
        .collect())
}
