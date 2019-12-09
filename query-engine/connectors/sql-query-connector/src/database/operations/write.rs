use crate::{error::SqlError, query_builder::write, QueryExt};
use connector_interface::{error::ConnectorError, *};
use prisma_models::*;
use quaint::error::Error as QueryError;

pub async fn create_record(
    conn: &dyn QueryExt,
    model: &ModelRef,
    args: WriteArgs,
) -> connector_interface::Result<GraphqlId> {
    let (insert, returned_id) = write::create_record(model, args.non_list_args().clone());

    let last_id = match conn.insert(insert).await {
        Ok(id) => id,
        Err(QueryError::UniqueConstraintViolation { field_name }) => {
            if field_name == "PRIMARY" {
                return Err(ConnectorError::UniqueConstraintViolation {
                    field_name: format!("{}.{}", model.name, model.fields().id().name),
                });
            } else {
                return Err(ConnectorError::UniqueConstraintViolation {
                    field_name: format!("{}.{}", model.name, field_name),
                });
            }
        }
        Err(QueryError::NullConstraintViolation { field_name }) => {
            if field_name == "PRIMARY" {
                return Err(ConnectorError::NullConstraintViolation {
                    field_name: format!("{}.{}", model.name, model.fields().id().name),
                });
            } else {
                return Err(ConnectorError::NullConstraintViolation {
                    field_name: format!("{}.{}", model.name, field_name),
                });
            }
        }
        Err(e) => return Err(SqlError::from(e).into()),
    };

    let id = match returned_id {
        Some(id) => id,
        None => GraphqlId::from(last_id.unwrap()),
    };

    Ok(id)
}

pub async fn update_records(
    conn: &dyn QueryExt,
    model: &ModelRef,
    where_: Filter,
    args: WriteArgs,
) -> connector_interface::Result<Vec<GraphqlId>> {
    let ids = conn.filter_ids(model, where_.clone()).await?;

    if ids.len() == 0 {
        return Ok(vec![]);
    }

    let updates = {
        let ids: Vec<&GraphqlId> = ids.iter().map(|id| &*id).collect();
        write::update_many(model, ids.as_slice(), args.non_list_args())?
    };

    for update in updates {
        conn.update(update).await.map_err(SqlError::from)?;
    }

    Ok(ids)
}

pub async fn delete_records(
    conn: &dyn QueryExt,
    model: &ModelRef,
    where_: Filter,
) -> connector_interface::Result<usize> {
    let ids = conn.filter_ids(model, where_.clone()).await?;
    let ids: Vec<&GraphqlId> = ids.iter().map(|id| &*id).collect();
    let count = ids.len();

    if count == 0 {
        return Ok(count);
    }

    for delete in write::delete_many(model, ids.as_slice()) {
        conn.delete(delete).await.map_err(SqlError::from)?;
    }

    Ok(count)
}

pub async fn connect(
    conn: &dyn QueryExt,
    field: &RelationFieldRef,
    parent_id: &GraphqlId,
    child_ids: &[GraphqlId],
) -> connector_interface::Result<()> {
    let query = write::create_relation_table_records(field, parent_id, child_ids);

    conn.execute(query).await.map_err(SqlError::from)?;
    Ok(())
}

pub async fn disconnect(
    conn: &dyn QueryExt,
    field: &RelationFieldRef,
    parent_id: &GraphqlId,
    child_ids: &[GraphqlId],
) -> connector_interface::Result<()> {
    let query = write::delete_relation_table_records(field, parent_id, child_ids);

    conn.execute(query).await.map_err(SqlError::from)?;
    Ok(())
}
