use crate::{
    query_builder::read::{self, ManyRelatedRecordsBaseQuery, ManyRelatedRecordsQueryBuilder},
    QueryExt, SqlError,
};

use connector_interface::*;
use prisma_models::*;
use quaint::ast::*;
use std::convert::TryFrom;

pub async fn get_single_record(
    conn: &dyn QueryExt,
    model: &ModelRef,
    filter: &Filter,
    selected_fields: &SelectedFields,
) -> crate::Result<Option<SingleRecord>> {
    let query = read::get_records(&model, selected_fields.columns(), filter);
    let field_names = selected_fields.names().map(String::from).collect();
    let idents: Vec<_> = selected_fields.types().collect();

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
) -> crate::Result<ManyRecords> {
    let field_names = selected_fields.names().map(String::from).collect();
    let idents: Vec<_> = selected_fields.types().collect();
    let query = read::get_records(model, selected_fields.columns(), query_arguments);

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
) -> crate::Result<ManyRecords>
where
    T: ManyRelatedRecordsQueryBuilder,
{
    let mut idents: Vec<_> = selected_fields.types().collect();
    idents.push(from_field.related_field().type_identifier_with_arity());
    idents.push(from_field.type_identifier_with_arity());

    let mut field_names: Vec<String> = selected_fields.names().map(String::from).collect();
    field_names.push(from_field.related_field().name.clone());
    field_names.push(from_field.name.clone());

    let can_skip_joins = from_field.relation_is_inlined_in_child() && !query_arguments.is_with_pagination();
    let relation = from_field.relation();

    let query = if can_skip_joins {
        let mut columns: Vec<_> = selected_fields.columns().collect();

        columns.extend(
            relation
                .columns_for_relation_side(from_field.relation_side.opposite())
                .into_iter()
                .map(|col| col.alias(SelectedFields::RELATED_MODEL_ALIAS))
                .collect()

        );

        columns.extend(
            relation
                .columns_for_relation_side(from_field.relation_side)
                .into_iter()
                .map(|col| col.alias(SelectedFields::PARENT_MODEL_ALIAS))
                .collect()
        );

        let model = from_field.related_model();

        let select = read::get_records(&model, columns.into_iter(), query_arguments)
            .and_where(from_field.relation_columns().pop().unwrap().in_selection(from_record_ids.to_owned()));

        Query::from(select)
    } else {
        let mut columns: Vec<_> = selected_fields.columns().collect();

        columns.extend(
            relation
                .columns_for_relation_side(from_field.relation_side.opposite())
                .into_iter()
                .map(|col| col.alias(SelectedFields::RELATED_MODEL_ALIAS)
                .table(Relation::TABLE_ALIAS))
                .collect()
        );

        columns.extend(
            relation
                .columns_for_relation_side(from_field.relation_side)
                .into_iter()
                .map(|col| col.alias(SelectedFields::PARENT_MODEL_ALIAS)
                .table(Relation::TABLE_ALIAS))
                .collect()
        );

        let is_with_pagination = query_arguments.is_with_pagination();
        let base = ManyRelatedRecordsBaseQuery::new(from_field, from_record_ids, query_arguments, columns);

        if is_with_pagination {
            T::with_pagination(base)
        } else {
            T::without_pagination(base)
        }
    };

    let records: crate::Result<Vec<Record>> = conn
        .filter(query, idents.as_slice())
        .await?
        .into_iter()
        .map(|mut row| {
            let parent_id = row.values.pop().ok_or(SqlError::ColumnDoesNotExist)?;

            // Relation id is always the second last value. We don't need it
            // here and we don't need it in the record.
            let _ = row.values.pop();

            let mut record = Record::from(row);
            record.set_parent_id(GraphqlId::try_from(parent_id)?);

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
) -> crate::Result<usize> {
    let query = read::count_by_model(model, query_arguments);
    let result = conn.find_int(query).await? as usize;

    Ok(result)
}
