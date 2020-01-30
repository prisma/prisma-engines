use crate::{error::SqlError, query_builder::write, QueryExt};
use connector_interface::*;
use itertools::Itertools;
use prisma_models::*;
use quaint::error::{DatabaseConstraint, Error as QueryError};
use std::convert::TryFrom;

pub async fn create_record(conn: &dyn QueryExt, model: &ModelRef, args: WriteArgs) -> crate::Result<RecordIdentifier> {
    let (insert, returned_id) = write::create_record(model, args);

    let result_set = match conn.insert(insert).await {
        Ok(result_set) => result_set,
        Err(QueryError::UniqueConstraintViolation { constraint }) => match constraint {
            DatabaseConstraint::Index(_) => {
                let fields = model
                    .primary_identifier()
                    .into_iter()
                    .map(|id_field| format!("{}.{}", model.name, id_field.name()));
                return Err(SqlError::UniqueConstraintViolation {
                    field_names: fields.collect(),
                });
            }
            DatabaseConstraint::Fields(fields) if fields.first().map(|s| s.as_str()) == Some("PRIMARY") => {
                let fields = model
                    .primary_identifier()
                    .into_iter()
                    .map(|id_field| format!("{}.{}", model.name, id_field.name()));
                return Err(SqlError::UniqueConstraintViolation {
                    field_names: fields.collect(),
                });
            }
            DatabaseConstraint::Fields(fields) => {
                let field_names = fields
                    .into_iter()
                    .map(|field_name| format!("{}.{}", model.name, field_name))
                    .collect();
                return Err(SqlError::UniqueConstraintViolation { field_names });
            }
        },
        Err(QueryError::NullConstraintViolation { constraint }) => match constraint {
            DatabaseConstraint::Index(_) => {
                let mut fields = model
                    .primary_identifier()
                    .into_iter()
                    .map(|id_field| format!("{}.{}", model.name, id_field.name()));
                return Err(SqlError::NullConstraintViolation {
                    field_name: fields.join(","),
                });
            }
            DatabaseConstraint::Fields(fields) if fields.first().map(|s| s.as_str()) == Some("PRIMARY") => {
                let mut fields = model
                    .primary_identifier()
                    .into_iter()
                    .map(|id_field| format!("{}.{}", model.name, id_field.name()));
                return Err(SqlError::NullConstraintViolation {
                    field_name: fields.join(","),
                });
            }
            DatabaseConstraint::Fields(fields) => {
                let field_name = fields
                    .into_iter()
                    .map(|field_name| format!("{}.{}", model.name, field_name))
                    .collect();
                return Err(SqlError::NullConstraintViolation { field_name });
            }
        },
        Err(e) => return Err(SqlError::from(e)),
    };

    match (returned_id, result_set.len(), result_set.last_insert_id()) {
        // All values provided in the write arrghs
        (Some(identifier), _, _) if !identifier.misses_autogen_value() => Ok(identifier),

        // PostgreSQL with a working RETURNING statement
        (_, n, _) if n > 0 => Ok(RecordIdentifier::try_from((&model.primary_identifier(), result_set))?),

        // We have an auto-incremented id that we got from MySQL or SQLite
        (Some(mut identifier), _, Some(num)) if identifier.misses_autogen_value() => {
            identifier.add_autogen_value(num as i64);
            Ok(identifier)
        }

        (_, _, _) => panic!("Could not figure out an ID in create"),
    }
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
        write::update_many(model, ids.as_slice(), args)?
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
