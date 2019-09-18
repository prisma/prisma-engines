use crate::{
    error::SqlError,
    query_builder::read::{ManyRelatedRecordsBaseQuery, ManyRelatedRecordsQueryBuilder, ReadQueryBuilder},
};

use connector_interface::{self, filter::RecordFinder, *};
use prisma_models::*;
use std::convert::TryFrom;

pub fn execute_get_single_record(
    conn: &mut dyn super::QueryExt,
    record_finder: &RecordFinder,
    selected_fields: &SelectedFields,
) -> crate::Result<Option<SingleRecord>> {
    let query = ReadQueryBuilder::get_records(record_finder.field.model(), selected_fields, record_finder);
    let field_names = selected_fields.names();
    let idents = selected_fields.type_identifiers();

    let record = (match conn.find(query, idents.as_slice()) {
        Ok(result) => Ok(Some(result)),
        Err(_e @ SqlError::RecordNotFoundForWhere(_)) => Ok(None),
        Err(e) => Err(e),
    })?
    .map(Record::from)
    .map(|record| SingleRecord { record, field_names });

    Ok(record)
}

pub fn execute_get_many_records(
    conn: &mut dyn super::QueryExt,
    model: ModelRef,
    query_arguments: QueryArguments,
    selected_fields: &SelectedFields,
) -> crate::Result<ManyRecords> {
    let field_names = selected_fields.names();
    let idents = selected_fields.type_identifiers();
    let query = ReadQueryBuilder::get_records(model, selected_fields, query_arguments);

    let records = conn
        .filter(query.into(), idents.as_slice())?
        .into_iter()
        .map(Record::from)
        .collect();

    Ok(ManyRecords { records, field_names })
}

pub fn execute_get_related_records<T: ManyRelatedRecordsQueryBuilder>(
    conn: &mut dyn super::QueryExt,
    from_field: RelationFieldRef,
    from_record_ids: &[GraphqlId],
    query_arguments: QueryArguments,
    selected_fields: &SelectedFields,
) -> crate::Result<ManyRecords> {
    let idents = selected_fields.type_identifiers();
    let field_names = selected_fields.names();

    let query = {
        let is_with_pagination = query_arguments.is_with_pagination();
        let base = ManyRelatedRecordsBaseQuery::new(from_field, from_record_ids, query_arguments, selected_fields);

        if is_with_pagination {
            T::with_pagination(base)
        } else {
            T::without_pagination(base)
        }
    };

    let mut records = Vec::new();
    for mut row in conn.filter(query, idents.as_slice())?.into_iter() {
        let parent_id = row.values.pop().ok_or(SqlError::ColumnDoesNotExist)?;

        // Relation id is always the second last value. We don't need it
        // here and we don't need it in the record.
        let _ = row.values.pop();

        let mut record = Record::from(row);
        record.add_parent_id(GraphqlId::try_from(parent_id)?);

        records.push(record)
    }

    //    let records = conn
    //        .filter(query, idents.as_slice())?
    //        .into_iter()
    //        .map(|mut row| {
    //            let parent_id = row.values.pop().ok_or(ConnectorError::ColumnDoesNotExist)?;
    //
    //            // Relation id is always the second last value. We don't need it
    //            // here and we don't need it in the record.
    //            let _ = row.values.pop();
    //
    //            let mut record = Record::from(row);
    //            record.add_parent_id(GraphqlId::try_from(parent_id)?);
    //
    //            Ok(record)
    //        })
    //        .collect();

    Ok(ManyRecords {
        records: records,
        field_names,
    })
}

pub fn execute_count_by_model(
    conn: &mut dyn super::QueryExt,
    model: ModelRef,
    query_arguments: QueryArguments,
) -> crate::Result<usize> {
    let query = ReadQueryBuilder::count_by_model(model, query_arguments);
    let result = conn.find_int(query)? as usize;

    Ok(result)
}
