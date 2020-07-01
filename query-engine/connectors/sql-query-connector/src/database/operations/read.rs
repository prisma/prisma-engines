use crate::{
    query_builder::{self, read},
    row::SqlRow,
    QueryExt, SqlError,
};
use connector_interface::*;
use datamodel::FieldArity;
use futures::stream::{FuturesUnordered, StreamExt};
use prisma_models::*;
use quaint::ast::*;

pub async fn get_single_record(
    conn: &dyn QueryExt,
    model: &ModelRef,
    filter: &Filter,
    selected_fields: &ModelProjection,
) -> crate::Result<Option<SingleRecord>> {
    let query = read::get_records(&model, selected_fields.as_columns(), filter);
    let field_names = selected_fields.db_names().map(String::from).collect();
    let idents: Vec<_> = selected_fields.type_identifiers_with_arities();

    let record = (match conn.find(query, idents.as_slice()).await {
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
) -> crate::Result<ManyRecords> {
    let reversed = query_arguments.needs_reversed_order();
    let field_names = selected_fields.db_names().map(String::from).collect();
    let idents: Vec<_> = selected_fields.type_identifiers_with_arities();
    let mut records = ManyRecords::new(field_names);

    if query_arguments.can_batch() {
        // We don't need to order in the database due to us ordering in this function.
        let order = query_arguments.order_by.take();

        let batches = query_arguments.batched();
        let mut futures = FuturesUnordered::new();

        for args in batches.into_iter() {
            let query = read::get_records(model, selected_fields.as_columns(), args);
            futures.push(conn.filter(query.into(), idents.as_slice()));
        }

        while let Some(result) = futures.next().await {
            for item in result?.into_iter() {
                records.push(Record::from(item))
            }
        }

        if let Some(ref order_by) = order {
            records.order_by(order_by)
        }
    } else {
        let query = read::get_records(model, selected_fields.as_columns(), query_arguments);

        for item in conn.filter(query.into(), idents.as_slice()).await?.into_iter() {
            records.push(Record::from(item))
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

    let relation = from_field.relation();
    let table = relation.as_table();

    let from_column_names: Vec<_> = from_field.related_field().m2m_column_names();
    let to_column_names: Vec<_> = from_field.m2m_column_names();
    let from_columns: Vec<Column<'static>> = from_column_names
        .iter()
        .map(|name| Column::from(name.clone()))
        .collect();

    // [DTODO] To verify: We might need chunked fetch here (too many parameters in the query).
    let select = Select::from_table(table)
        .columns(from_column_names.into_iter().chain(to_column_names.into_iter()))
        .so_that(query_builder::conditions(&from_columns, from_record_ids));

    let parent_model_id = from_field.model().primary_identifier();
    let child_model_id = from_field.related_model().primary_identifier();

    let from_sfs: Vec<_> = parent_model_id.scalar_fields().collect();
    let to_sfs: Vec<_> = child_model_id.scalar_fields().collect();

    // first parent id, then child id
    Ok(conn
        .filter(select.into(), idents.as_slice())
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

pub async fn aggregate(
    conn: &dyn QueryExt,
    model: &ModelRef,
    aggregators: Vec<Aggregator>,
    query_arguments: QueryArguments,
) -> crate::Result<Vec<AggregationResult>> {
    let query = read::aggregate(model, &aggregators, query_arguments);
    let idents = extract_query_idents(&aggregators);

    let mut rows = conn.filter(query.into(), idents.as_slice()).await?;
    let row = rows
        .pop()
        .expect("Expected exactly one return row for aggregation query.");

    Ok(parse_aggregation_row(&aggregators, row))
}

fn extract_query_idents(aggregators: &[Aggregator]) -> Vec<(TypeIdentifier, FieldArity)> {
    aggregators
        .iter()
        .flat_map(|aggregator| match aggregator {
            Aggregator::Count => vec![(TypeIdentifier::Int, FieldArity::Required)],
            Aggregator::Average(fields) => map_aggregator_fields(&fields, Some(TypeIdentifier::Float)),
            Aggregator::Sum(fields) => map_aggregator_fields(&fields, None),
            Aggregator::Min(fields) => map_aggregator_fields(&fields, None),
            Aggregator::Max(fields) => map_aggregator_fields(&fields, None),
        })
        .collect()
}

fn map_aggregator_fields(
    fields: &[ScalarFieldRef],
    fixed_type: Option<TypeIdentifier>,
) -> Vec<(TypeIdentifier, FieldArity)> {
    fields
        .into_iter()
        .map(|f| {
            (
                fixed_type.clone().unwrap_or(f.type_identifier.clone()),
                FieldArity::Required,
            )
        })
        .collect()
}

fn parse_aggregation_row(aggregators: &[Aggregator], row: SqlRow) -> Vec<AggregationResult> {
    let mut values = row.values;
    values.reverse();

    aggregators
        .iter()
        .flat_map(|aggregator| match aggregator {
            Aggregator::Count => vec![AggregationResult::Count(values.pop().unwrap())],

            Aggregator::Average(fields) => fields
                .iter()
                .map(|field| AggregationResult::Average(field.clone(), values.pop().unwrap()))
                .collect(),

            Aggregator::Sum(fields) => fields
                .iter()
                .map(|field| AggregationResult::Sum(field.clone(), values.pop().unwrap()))
                .collect(),

            Aggregator::Min(fields) => fields
                .iter()
                .map(|field| AggregationResult::Min(field.clone(), values.pop().unwrap()))
                .collect(),

            Aggregator::Max(fields) => fields
                .iter()
                .map(|field| AggregationResult::Max(field.clone(), values.pop().unwrap()))
                .collect(),
        })
        .collect()
}
