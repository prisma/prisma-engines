use crate::{
    database::{SqlCapabilities, SqlDatabase},
    error::SqlError,
    query_builder::read::{ManyRelatedRecordsBaseQuery, ManyRelatedRecordsQueryBuilder, ReadQueryBuilder},
    Transactional,
};

use connector_interface::{self, filter::RecordFinder, *};
use itertools::Itertools;
use prisma_models::*;
use std::convert::TryFrom;

struct ScalarListElement {
    record_id: GraphqlId,
    value: PrismaValue,
}

impl<T> ManagedDatabaseReader for SqlDatabase<T>
where
    T: Transactional + SqlCapabilities,
{
    fn get_single_record(
        &self,
        record_finder: &RecordFinder,
        selected_fields: &SelectedFields,
    ) -> connector_interface::Result<Option<SingleRecord>> {
        let db_name = &record_finder.field.model().internal_data_model().db_name;
        let result = self.executor.with_transaction(db_name, |transaction| {
            execute_get_single_record(transaction, record_finder, selected_fields)
        })?;
        Ok(result)
    }

    fn get_many_records(
        &self,
        model: ModelRef,
        query_arguments: QueryArguments,
        selected_fields: &SelectedFields,
    ) -> connector_interface::Result<ManyRecords> {
        let db_name = &model.internal_data_model().db_name;
        let result = self.executor.with_transaction(db_name, |transaction| {
            execute_get_many_records(transaction, model, query_arguments, selected_fields)
        })?;

        Ok(result)
    }

    fn get_related_records(
        &self,
        from_field: RelationFieldRef,
        from_record_ids: &[GraphqlId],
        query_arguments: QueryArguments,
        selected_fields: &SelectedFields,
    ) -> connector_interface::Result<ManyRecords> {
        let db_name = &from_field.model().internal_data_model().db_name;
        let result = self.executor.with_transaction(db_name, |transaction| {
            execute_get_related_records::<T::ManyRelatedRecordsBuilder>(
                transaction,
                from_field,
                from_record_ids,
                query_arguments,
                selected_fields,
            )
        })?;

        Ok(result)
    }

    fn count_by_model(&self, model: ModelRef, query_arguments: QueryArguments) -> connector_interface::Result<usize> {
        let db_name = &model.internal_data_model().db_name;
        let result = self.executor.with_transaction(db_name, |transaction| {
            execute_count_by_model(transaction, model, query_arguments)
        })?;
        Ok(result)
    }

    fn count_by_table(&self, database: &str, table: &str) -> connector_interface::Result<usize> {
        let query = ReadQueryBuilder::count_by_table(database, table);

        let result = self
            .executor
            .with_transaction(database, |conn| conn.find_int(query))
            .map(|count| count as usize)?;

        Ok(result)
    }

    fn get_scalar_list_values_by_record_ids(
        &self,
        list_field: ScalarFieldRef,
        record_ids: Vec<GraphqlId>,
    ) -> connector_interface::Result<Vec<ScalarListValues>> {
        let db_name = &list_field.model().internal_data_model().db_name;
        let type_identifier = list_field.type_identifier;
        let query = ReadQueryBuilder::get_scalar_list_values_by_record_ids(list_field, record_ids);

        let results: Vec<ScalarListElement> = self.executor.with_transaction(db_name, |conn| {
            let rows = conn.filter(query.into(), &[TypeIdentifier::GraphQLID, type_identifier])?;

            rows.into_iter()
                .map(|row| {
                    let mut iter = row.values.into_iter();

                    let record_id = iter.next().ok_or(SqlError::ColumnDoesNotExist)?;
                    let value = iter.next().ok_or(SqlError::ColumnDoesNotExist)?;

                    Ok(ScalarListElement {
                        record_id: GraphqlId::try_from(record_id)?,
                        value,
                    })
                })
                .collect()
        })?;

        let mut list_values = Vec::new();

        for (record_id, elements) in &results.into_iter().group_by(|ele| ele.record_id.clone()) {
            let values = ScalarListValues {
                record_id,
                values: elements.into_iter().map(|e| e.value).collect(),
            };
            list_values.push(values);
        }

        Ok(list_values)
    }
}

pub fn execute_get_single_record(
    transaction: &mut dyn super::Transaction,
    record_finder: &RecordFinder,
    selected_fields: &SelectedFields,
) -> crate::Result<Option<SingleRecord>> {
    let query = ReadQueryBuilder::get_records(record_finder.field.model(), selected_fields, record_finder);
    let field_names = selected_fields.names();
    let idents = selected_fields.type_identifiers();

    let record = (match transaction.find(query, idents.as_slice()) {
        Ok(result) => Ok(Some(result)),
        Err(_e @ SqlError::RecordNotFoundForWhere(_)) => Ok(None),
        Err(e) => Err(e),
    })?
    .map(Record::from)
    .map(|record| SingleRecord { record, field_names });

    Ok(record)
}

pub fn execute_get_many_records(
    transaction: &mut dyn super::Transaction,
    model: ModelRef,
    query_arguments: QueryArguments,
    selected_fields: &SelectedFields,
) -> crate::Result<ManyRecords> {
    let field_names = selected_fields.names();
    let idents = selected_fields.type_identifiers();
    let query = ReadQueryBuilder::get_records(model, selected_fields, query_arguments);

    let records = transaction
        .filter(query.into(), idents.as_slice())?
        .into_iter()
        .map(Record::from)
        .collect();

    Ok(ManyRecords { records, field_names })
}

pub fn execute_get_related_records<T: ManyRelatedRecordsQueryBuilder>(
    transaction: &mut dyn super::Transaction,
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
    for mut row in transaction.filter(query, idents.as_slice())?.into_iter() {
        let parent_id = row.values.pop().ok_or(SqlError::ColumnDoesNotExist)?;

        // Relation id is always the second last value. We don't need it
        // here and we don't need it in the record.
        let _ = row.values.pop();

        let mut record = Record::from(row);
        record.add_parent_id(GraphqlId::try_from(parent_id)?);

        records.push(record)
    }

    //    let records = transaction
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
    transaction: &mut dyn super::Transaction,
    model: ModelRef,
    query_arguments: QueryArguments,
) -> crate::Result<usize> {
    let query = ReadQueryBuilder::count_by_model(model, query_arguments);
    let result = transaction.find_int(query)? as usize;

    Ok(result)
}
