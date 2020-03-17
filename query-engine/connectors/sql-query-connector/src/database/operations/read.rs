use crate::{
    query_builder::{self, read},
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
    selected_fields: &SelectedFields,
) -> crate::Result<Option<SingleRecord>> {
    let query = read::get_records(&model, selected_fields.columns(), filter);
    let field_names = selected_fields.db_names().map(String::from).collect();
    let idents: Vec<_> = selected_fields.types().collect();

    let record = (match conn.find(query, idents.as_slice()).await {
        Ok(result) => Ok(Some(result)),
        Err(_e @ SqlError::RecordNotFoundForWhere(_)) => Ok(None),
        Err(_e @ SqlError::RecordDoesNotExist) => Ok(None),
        Err(e) => Err(e),
    })?
    .map(Record::from)
    .map(|record| SingleRecord {
        record,
        field_names,
    });

    Ok(record)
}

pub async fn get_many_records(
    conn: &dyn QueryExt,
    model: &ModelRef,
    mut query_arguments: QueryArguments,
    selected_fields: &SelectedFields,
) -> crate::Result<ManyRecords> {
    let field_names = selected_fields.db_names().map(String::from).collect();
    let idents: Vec<_> = selected_fields.types().collect();
    let mut records = ManyRecords::new(field_names);

    if query_arguments.can_batch() {
        // We don't need to order in the database due to us ordering in this
        // function.
        let order = query_arguments.order_by.take();

        let batches = query_arguments.batched();
        let mut futures = FuturesUnordered::new();

        for args in batches.into_iter() {
            let query = read::get_records(model, selected_fields.columns(), args);
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
        let query = read::get_records(model, selected_fields.columns(), query_arguments);

        for item in conn
            .filter(query.into(), idents.as_slice())
            .await?
            .into_iter()
        {
            records.push(Record::from(item))
        }
    }

    Ok(records)
}

pub async fn get_related_m2m_record_ids(
    conn: &dyn QueryExt,
    from_field: &RelationFieldRef,
    from_record_ids: &[RecordProjection],
) -> crate::Result<Vec<(RecordProjection, RecordProjection)>> {
    let mut idents = vec![];
    idents.extend(from_field.type_identifiers_with_arities());
    idents.extend(from_field.related_field().type_identifiers_with_arities());

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
        .columns(
            from_column_names
                .into_iter()
                .chain(to_column_names.into_iter()),
        )
        .so_that(query_builder::conditions(&from_columns, from_record_ids));

    let parent_model_id = from_field.model().primary_identifier();
    let child_model_id = from_field.related_model().primary_identifier();

    let from_dsfs: Vec<_> = parent_model_id.data_source_fields().collect();
    let to_dsfs: Vec<_> = child_model_id.data_source_fields().collect();

    // first parent id, then child id
    Ok(conn
        .filter(select.into(), idents.as_slice())
        .await?
        .into_iter()
        .map(|row| {
            let mut values = row.values;

            let child_values = values.split_off(from_dsfs.len());
            let parent_values = values;

            let p: RecordProjection = from_dsfs
                .iter()
                .zip(parent_values)
                .map(|(dsf, val)| (dsf.clone(), val))
                .collect::<Vec<_>>()
                .into();

            let c: RecordProjection = to_dsfs
                .iter()
                .zip(child_values)
                .map(|(dsf, val)| (dsf.clone(), val))
                .collect::<Vec<_>>()
                .into();

            (p, c)
        })
        .collect())
}

pub async fn count_by_model(
    conn: &dyn QueryExt,
    model: &ModelRef,
    query_arguments: QueryArguments,
) -> crate::Result<usize> {
    let query = read::count_by_model(model, query_arguments);
    let result = conn.find_int(query).await? as usize;

    Ok(result)
}
