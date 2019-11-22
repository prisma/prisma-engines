use crate::{error::SqlError, query_builder::write, QueryExt};
use connector_interface::*;
use prisma_models::*;
use quaint::error::Error as QueryError;

pub async fn create_record(
    conn: &dyn QueryExt,
    model: &ModelRef,
    args: WriteArgs,
) -> crate::Result<GraphqlId> {
    let (insert, returned_id) = write::create_record(model, args.non_list_args().clone());

    let last_id = match conn.insert(insert).await {
        Ok(id) => id,
        Err(QueryError::UniqueConstraintViolation { field_name }) => {
            if field_name == "PRIMARY" {
                return Err(SqlError::UniqueConstraintViolation {
                    field_name: format!("{}.{}", model.name, model.fields().id().name),
                });
            } else {
                return Err(SqlError::UniqueConstraintViolation {
                    field_name: format!("{}.{}", model.name, field_name),
                });
            }
        }
        Err(QueryError::NullConstraintViolation { field_name }) => {
            if field_name == "PRIMARY" {
                return Err(SqlError::NullConstraintViolation {
                    field_name: format!("{}.{}", model.name, model.fields().id().name),
                });
            } else {
                return Err(SqlError::NullConstraintViolation {
                    field_name: format!("{}.{}", model.name, field_name),
                });
            }
        }
        Err(e) => return Err(SqlError::from(e)),
    };

    let id = match returned_id {
        Some(id) => id,
        None => GraphqlId::from(last_id.unwrap()),
    };

    for (field_name, list_value) in args.list_args() {
        let field = model.fields().find_from_scalar(field_name.as_ref()).unwrap();
        let table = field.scalar_list_table();

        if let Some(insert) = write::create_scalar_list_value(table.table(), &list_value, &id) {
            conn.insert(insert).await?;
        }
    }

    Ok(id)
}

pub async fn update_records(
    conn: &dyn QueryExt,
    model: &ModelRef,
    where_: Filter,
    args: WriteArgs,
) -> crate::Result<Vec<GraphqlId>> {
    let ids = conn.filter_ids(model, where_.clone()).await?;

    if ids.len() == 0 {
        return Ok(vec![]);
    }

    let updates = {
        let ids: Vec<&GraphqlId> = ids.iter().map(|id| &*id).collect();
        write::update_many(model, ids.as_slice(), args.non_list_args())?
    };

    for update in updates {
        conn.update(update).await?;
    }


    for (field_name, list_value) in args.list_args() {
        let field = model.fields().find_from_scalar(field_name.as_ref()).unwrap();
        let table = field.scalar_list_table();
        let (deletes, inserts) = write::update_scalar_list_values(&table, &list_value, ids.to_vec());

        for delete in deletes {
            conn.delete(delete).await?;
        }

        for insert in inserts {
            conn.insert(insert).await?;
        }
    }

    Ok(ids)
}

pub async fn delete_records(
    conn: &dyn QueryExt,
    model: &ModelRef,
    where_: Filter,
) -> crate::Result<usize> {
    let ids = conn.filter_ids(model, where_.clone()).await?;
    let ids: Vec<&GraphqlId> = ids.iter().map(|id| &*id).collect();
    let count = ids.len();

    if count == 0 {
        return Ok(count);
    }

    for delete in write::delete_many(model, ids.as_slice()) {
        conn.delete(delete).await?;
    }

    Ok(count)
}

pub async fn connect(
    conn: &dyn QueryExt,
    field: &RelationFieldRef,
    parent_id: &GraphqlId,
    child_ids: &[GraphqlId],
) -> crate::Result<()> {
    let query = write::create_relation_table_records(field, parent_id, child_ids);

    conn.execute(query).await?;
    Ok(())
}

pub async fn disconnect(
    conn: &dyn QueryExt,
    field: &RelationFieldRef,
    parent_id: &GraphqlId,
    child_ids: &[GraphqlId],
) -> crate::Result<()> {
    let query = write::delete_relation_table_records(field, parent_id, child_ids);

    conn.execute(query).await?;
    Ok(())
}
