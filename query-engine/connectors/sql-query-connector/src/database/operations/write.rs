use crate::{query_builder::write, QueryExt};
use connector_interface::*;
use prisma_models::*;

pub async fn create_record(_conn: &dyn QueryExt, model: &ModelRef, args: WriteArgs) -> crate::Result<RecordIdentifier> {
    let (_insert, _returned_id) = write::create_record(model, args);

    // let last_id = match conn.insert(insert).await {
    //     Ok(id) => id,
    //     Err(QueryError::UniqueConstraintViolation { field_name }) => {
    //         if field_name == "PRIMARY" {
    //             return Err(SqlError::UniqueConstraintViolation {
    //                 field_name: format!("{}.{}", model.name, model.fields().id().name),
    //             });
    //         } else {
    //             return Err(SqlError::UniqueConstraintViolation {
    //                 field_name: format!("{}.{}", model.name, field_name),
    //             });
    //         }
    //     }
    //     Err(QueryError::NullConstraintViolation { field_name }) => {
    //         if field_name == "PRIMARY" {
    //             return Err(SqlError::NullConstraintViolation {
    //                 field_name: format!("{}.{}", model.name, model.fields().id().name),
    //             });
    //         } else {
    //             return Err(SqlError::NullConstraintViolation {
    //                 field_name: format!("{}.{}", model.name, field_name),
    //             });
    //         }
    //     }
    //     Err(e) => return Err(SqlError::from(e)),
    // };

    // let id = match returned_id {
    //     Some(id) => id,
    //     None => RecordIdentifier::from(last_id.unwrap()),
    // };

    // Ok(id)

    todo!()
}

pub async fn update_records(
    _conn: &dyn QueryExt,
    _model: &ModelRef,
    _where_: Filter,
    _args: WriteArgs,
) -> crate::Result<Vec<RecordIdentifier>> {
    // let ids = conn.filter_ids(model, where_.clone()).await?;

    // if ids.len() == 0 {
    //     return Ok(vec![]);
    // }

    // let updates = {
    //     let ids: Vec<&RecordIdentifier> = ids.iter().map(|id| &*id).collect();
    //     write::update_many(model, ids.as_slice(), args.non_list_args())?
    // };

    // for update in updates {
    //     conn.update(update).await?;
    // }

    // Ok(ids)

    todo!()
}

pub async fn delete_records(_conn: &dyn QueryExt, _model: &ModelRef, _where_: Filter) -> crate::Result<usize> {
    // let ids = conn.filter_ids(model, where_.clone()).await?;
    // let ids: Vec<&RecordIdentifier> = ids.iter().map(|id| &*id).collect();
    // let count = ids.len();

    // if count == 0 {
    //     return Ok(count);
    // }

    // for delete in write::delete_many(model, ids.as_slice()) {
    //     conn.delete(delete).await?;
    // }

    // Ok(count)

    todo!()
}

pub async fn connect(
    _conn: &dyn QueryExt,
    _field: &RelationFieldRef,
    _parent_id: &RecordIdentifier,
    _child_ids: &[RecordIdentifier],
) -> crate::Result<()> {
    // let query = write::create_relation_table_records(field, parent_id, child_ids);

    // conn.execute(query).await?;
    // Ok(())

    todo!()
}

pub async fn disconnect(
    _conn: &dyn QueryExt,
    _field: &RelationFieldRef,
    _parent_id: &RecordIdentifier,
    _child_ids: &[RecordIdentifier],
) -> crate::Result<()> {
    // let query = write::delete_relation_table_records(field, parent_id, child_ids);

    // conn.execute(query).await?;
    // Ok(())

    todo!()
}
