use crate::{query_builder::write, QueryExt, error::SqlError};
use connector_interface::*;
use prisma_models::*;
use quaint::error::{Error as QueryError, DatabaseConstraint};
use itertools::Itertools;

pub async fn create_record(conn: &dyn QueryExt, model: &ModelRef, args: WriteArgs) -> crate::Result<RecordIdentifier> {
    let (insert, _returned_id) = write::create_record(model, args);

    let _last_id = match conn.insert(insert).await {
        Ok(id) => id,
        Err(QueryError::UniqueConstraintViolation { constraint }) => {
            match constraint {
                DatabaseConstraint::Index(_) => {
                    let fields = model.identifier().into_iter().map(|id| format!("{}.{}", model.name, id.name));
                    return Err(SqlError::UniqueConstraintViolation { field_names: fields.collect() });
                },
                DatabaseConstraint::Fields(fields) if fields.first().map(|s| s.as_str()) == Some("PRIMARY") => {
                    let fields = model.identifier().into_iter().map(|id| format!("{}.{}", model.name, id.name));
                    return Err(SqlError::UniqueConstraintViolation { field_names: fields.collect() });
                },
                DatabaseConstraint::Fields(fields) => {
                    let field_names = fields.into_iter().map(|field_name| format!("{}.{}", model.name, field_name)).collect();
                    return Err(SqlError::UniqueConstraintViolation { field_names })
                }
            }
        }
        Err(QueryError::NullConstraintViolation { constraint }) => {
            match constraint {
                DatabaseConstraint::Index(_) => {
                    let mut fields = model.identifier().into_iter().map(|id| format!("{}.{}", model.name, id.name));
                    return Err(SqlError::NullConstraintViolation { field_name: fields.join(",") });
                },
                DatabaseConstraint::Fields(fields) if fields.first().map(|s| s.as_str()) == Some("PRIMARY") => {
                    let mut fields = model.identifier().into_iter().map(|id| format!("{}.{}", model.name, id.name));
                    return Err(SqlError::NullConstraintViolation { field_name: fields.join(",") });
                },
                DatabaseConstraint::Fields(fields) => {
                    let field_name = fields.into_iter().map(|field_name| format!("{}.{}", model.name, field_name)).collect();
                    return Err(SqlError::NullConstraintViolation { field_name })
                }
            }
        }
        Err(e) => return Err(SqlError::from(e)),
    };

    todo!()
    /*
    let id = match (returned_id, last_id) {
        (Some(id), _) => id,
        (_, Some(id)) =>
        None => RecordIdentifier::from(last_id.unwrap()),
    };

    Ok(id)
    */
}

pub async fn update_records(
    conn: &dyn QueryExt,
    model: &ModelRef,
    where_: Filter,
    args: WriteArgs,
) -> crate::Result<Vec<RecordIdentifier>> {
    let ids = conn.filter_ids(model, where_.clone()).await?;

    if ids.len() == 0 {
        return Ok(vec![]);
    }

    let updates = {
        let ids: Vec<&RecordIdentifier> = ids.iter().map(|id| &*id).collect();
        write::update_many(model, ids.as_slice(), &args)?
    };

    for update in updates {
        conn.query(update).await?;
    }

    Ok(ids)
}

pub async fn delete_records(conn: &dyn QueryExt, model: &ModelRef, where_: Filter) -> crate::Result<usize> {
    let ids = conn.filter_ids(model, where_.clone()).await?;
    let ids: Vec<&RecordIdentifier> = ids.iter().map(|id| &*id).collect();
    let count = ids.len();

    if count == 0 {
        return Ok(count);
    }

    for delete in write::delete_many(model, ids.as_slice()) {
        conn.query(delete).await?;
    }

    Ok(count)
}

pub async fn connect(
    conn: &dyn QueryExt,
    field: &RelationFieldRef,
    parent_id: &RecordIdentifier,
    child_ids: &[RecordIdentifier],
) -> crate::Result<()> {
    let query = write::create_relation_table_records(field, parent_id, child_ids);

    conn.query(query).await?;
    Ok(())
}

pub async fn disconnect(
    conn: &dyn QueryExt,
    field: &RelationFieldRef,
    parent_id: &RecordIdentifier,
    child_ids: &[RecordIdentifier],
) -> crate::Result<()> {
    let query = write::delete_relation_table_records(field, parent_id, child_ids);
    conn.delete(query).await?;

    Ok(())
}
