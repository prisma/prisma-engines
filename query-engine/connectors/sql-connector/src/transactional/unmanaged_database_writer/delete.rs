use crate::{
    error::SqlError,
    query_builder::{DeleteActions, WriteQueryBuilder},
    Transaction,
};
use connector_interface::{error::RecordFinderInfo, filter::RecordFinder};
use prisma_models::{GraphqlId, RelationFieldRef, SingleRecord};
use std::sync::Arc;

/// A top level delete that removes one record. Violating any relations or a
/// non-existing record will cause an error.
///
/// Will return the deleted record if the delete was successful.
pub fn execute(conn: &mut dyn Transaction, record_finder: &RecordFinder) -> crate::Result<SingleRecord> {
    let model = record_finder.field.model();
    let record = conn.find_record(record_finder)?;
    let id = record.collect_id(&model.fields().id().name).unwrap();

    DeleteActions::check_relation_violations(Arc::clone(&model), &[&id], |select| {
        let ids = conn.select_ids(select)?;
        Ok(ids.into_iter().next())
    })?;

    for delete in WriteQueryBuilder::delete_many(model, &[&id]) {
        conn.delete(delete)?;
    }

    Ok(record)
}
