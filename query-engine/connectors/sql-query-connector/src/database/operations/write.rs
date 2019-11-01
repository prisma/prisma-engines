use crate::{error::SqlError, query_builder::WriteQueryBuilder, QueryExt};
use connector_interface::{error::ConnectorError, *};
use prisma_models::*;
use prisma_query::error::Error as QueryError;
use std::sync::Arc;

pub async fn create_record(
    conn: &dyn QueryExt,
    model: ModelRef,
    args: WriteArgs,
) -> connector_interface::Result<GraphqlId> {
    let (insert, returned_id) = WriteQueryBuilder::create_record(Arc::clone(&model), args.non_list_args().clone());

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

    for (field_name, list_value) in args.list_args() {
        let field = model.fields().find_from_scalar(field_name.as_ref()).unwrap();
        let table = field.scalar_list_table();

        if let Some(insert) = WriteQueryBuilder::create_scalar_list_value(table.table(), &list_value, &id) {
            conn.insert(insert).await.map_err(SqlError::from)?;
        }
    }

    Ok(id)
}

pub async fn update_records(
    conn: &dyn QueryExt,
    model: ModelRef,
    where_: Filter,
    args: WriteArgs,
) -> connector_interface::Result<Vec<GraphqlId>> {
    let ids = conn.filter_ids(Arc::clone(&model), where_.clone()).await?;

    if ids.len() == 0 {
        return Ok(vec![]);
    }

    let updates = {
        let ids: Vec<&GraphqlId> = ids.iter().map(|id| &*id).collect();
        WriteQueryBuilder::update_many(Arc::clone(&model), ids.as_slice(), args.non_list_args())?
    };

    for update in updates {
        conn.update(update).await.map_err(SqlError::from)?;
    }

    for (field_name, list_value) in args.list_args() {
        let field = model.fields().find_from_scalar(field_name.as_ref()).unwrap();
        let table = field.scalar_list_table();
        let (deletes, inserts) = WriteQueryBuilder::update_scalar_list_values(&table, &list_value, ids.to_vec());

        for delete in deletes {
            conn.delete(delete).await.map_err(SqlError::from)?;
        }

        for insert in inserts {
            conn.insert(insert).await.map_err(SqlError::from)?;
        }
    }

    Ok(ids)
}

pub async fn delete_records(
    conn: &dyn QueryExt,
    model: ModelRef,
    where_: Filter,
) -> connector_interface::Result<usize> {
    let ids = conn.filter_ids(Arc::clone(&model), where_.clone()).await?;
    let ids: Vec<&GraphqlId> = ids.iter().map(|id| &*id).collect();
    let count = ids.len();

    if count == 0 {
        return Ok(count);
    }

    for delete in WriteQueryBuilder::delete_many(model, ids.as_slice()) {
        conn.delete(delete).await.map_err(SqlError::from)?;
    }

    Ok(count)
}

pub async fn connect(
    conn: &dyn QueryExt,
    field: RelationFieldRef,
    parent_id: &GraphqlId,
    child_id: &GraphqlId,
) -> connector_interface::Result<()> {
    let query = WriteQueryBuilder::create_relation(field, parent_id, child_id);
    conn.execute(query).await.map_err(SqlError::from)?;

    Ok(())
}

pub async fn disconnect(
    conn: &dyn QueryExt,
    field: RelationFieldRef,
    parent_id: &GraphqlId,
    child_id: &GraphqlId,
) -> connector_interface::Result<()> {
    let query = WriteQueryBuilder::delete_relation(field, parent_id, child_id);
    conn.execute(query).await.map_err(SqlError::from)?;

    Ok(())
}

pub async fn set(
    conn: &dyn QueryExt,
    field: RelationFieldRef,
    parent_id: GraphqlId,
    child_ids: Vec<GraphqlId>,
) -> connector_interface::Result<()> {
    let query = WriteQueryBuilder::delete_relation_by_parent(Arc::clone(&field), &parent_id);
    conn.execute(query).await.map_err(SqlError::from)?;

    // TODO: we can avoid the multiple roundtrips in some cases
    for child_id in &child_ids {
        connect(conn, Arc::clone(&field), &parent_id, child_id).await?;
    }
    Ok(())
}
