use crate::{
    query_builder::read::{self, ManyRelatedRecordsBaseQuery, ManyRelatedRecordsQueryBuilder},
    QueryExt, SqlError,
};
use connector_interface::{error::ConnectorError, *};
use prisma_models::*;
use quaint::ast::*;
use std::convert::TryFrom;

pub async fn get_single_record(
    conn: &dyn QueryExt,
    record_finder: &RecordFinder,
    selected_fields: &SelectedFields,
) -> connector_interface::Result<Option<SingleRecord>> {
    let model = record_finder.field.model();
    let query = read::get_records(&model, selected_fields, record_finder);
    let field_names = selected_fields.names();
    let idents = selected_fields.types();

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
    query_arguments: QueryArguments,
    selected_fields: &SelectedFields,
) -> connector_interface::Result<ManyRecords> {
    let field_names = selected_fields.names();
    let idents = selected_fields.types();
    let query = read::get_records(model, selected_fields, query_arguments);

    let records = conn
        .filter(query.into(), idents.as_slice())
        .await?
        .into_iter()
        .map(Record::from)
        .collect();

    Ok(ManyRecords { records, field_names })
}

pub async fn get_related_records<T>(
    conn: &dyn QueryExt,
    from_field: &RelationFieldRef,
    from_record_ids: &[GraphqlId],
    query_arguments: QueryArguments,
    selected_fields: &SelectedFields,
) -> connector_interface::Result<ManyRecords>
where
    T: ManyRelatedRecordsQueryBuilder,
{
    let idents = selected_fields.types();
    let field_names = selected_fields.names();

    let can_skip_joins = from_field.relation_is_inlined_in_child() && !query_arguments.is_with_pagination();

    let query = if can_skip_joins {
        let model = from_field.related_model();

        let select = read::get_records(&model, selected_fields, query_arguments)
            .and_where(from_field.relation_column().in_selection(from_record_ids.to_owned()));

        Query::from(select)
    } else {
        let is_with_pagination = query_arguments.is_with_pagination();
        let base = ManyRelatedRecordsBaseQuery::new(from_field, from_record_ids, query_arguments, selected_fields);

        if is_with_pagination {
            T::with_pagination(base)
        } else {
            T::without_pagination(base)
        }
    };

    let records: Result<Vec<Record>> = conn
        .filter(query, idents.as_slice())
        .await?
        .into_iter()
        .map(|mut row| {
            let parent_id = row.values.pop().ok_or(ConnectorError::ColumnDoesNotExist)?;

            // Relation id is always the second last value. We don't need it
            // here and we don't need it in the record.
            let _ = row.values.pop();

            let mut record = Record::from(row);
            record.add_parent_id(GraphqlId::try_from(parent_id)?);

            Ok(record)
        })
        .collect();

    Ok(ManyRecords {
        records: records?,
        field_names,
    })
}

pub async fn count_by_model(
    conn: &dyn QueryExt,
    model: &ModelRef,
    query_arguments: QueryArguments,
) -> connector_interface::Result<usize> {
    let query = read::count_by_model(model, query_arguments);
    let result = conn.find_int(query).await? as usize;

    Ok(result)
}
