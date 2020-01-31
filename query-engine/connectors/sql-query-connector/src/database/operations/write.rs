use crate::{error::SqlError, query_builder::write, QueryExt};
use connector_interface::*;
use prisma_models::*;
use quaint::error::{DatabaseConstraint, ErrorKind};
use std::convert::TryFrom;

pub async fn create_record(conn: &dyn QueryExt, model: &ModelRef, args: WriteArgs) -> crate::Result<GraphqlId> {
    let (insert, returned_id) = write::create_record(model, args.non_list_args().clone());

    let result_set = match conn.insert(insert).await {
        Ok(id) => id,
        Err(e) => match e.kind() {
            ErrorKind::UniqueConstraintViolation { constraint } => match constraint {
                DatabaseConstraint::Index(_) => {
                    let field_names = vec![format!("{}.{}", model.name, model.fields().id().name)];

                    return Err(SqlError::UniqueConstraintViolation {
                        constraint: field_names.into(),
                    });
                }
                DatabaseConstraint::Fields(fields) if fields.first().map(|s| s.as_str()) == Some("PRIMARY") => {
                    let field_names = vec![format!("{}.{}", model.name, model.fields().id().name)];

                    return Err(SqlError::UniqueConstraintViolation {
                        constraint: field_names.into(),
                    });
                }

                DatabaseConstraint::Fields(fields) => {
                    let field_names: Vec<String> = fields
                        .into_iter()
                        .map(|field_name| format!("{}.{}", model.name, field_name))
                        .collect();

                    return Err(SqlError::UniqueConstraintViolation {
                        constraint: field_names.into(),
                    });
                }
            },
            ErrorKind::NullConstraintViolation { constraint } => match constraint {
                DatabaseConstraint::Index(_) => {
                    let field_names = vec![format!("{}.{}", model.name, model.fields().id().name)];

                    return Err(SqlError::NullConstraintViolation {
                        constraint: field_names.into(),
                    });
                }
                DatabaseConstraint::Fields(fields) if fields.first().map(|s| s.as_str()) == Some("PRIMARY") => {
                    let field_names = vec![format!("{}.{}", model.name, model.fields().id().name)];

                    return Err(SqlError::NullConstraintViolation {
                        constraint: field_names.into(),
                    });
                }

                DatabaseConstraint::Fields(fields) => {
                    let field_names: Vec<String> = fields
                        .into_iter()
                        .map(|field_name| format!("{}.{}", model.name, field_name))
                        .collect();

                    return Err(SqlError::NullConstraintViolation {
                        constraint: field_names.into(),
                    });
                }
            },
            _ => return Err(SqlError::from(e)),
        },
    };

    let last_id = result_set.last_insert_id();

    match (returned_id, result_set.into_single(), last_id) {
        // Id is already in the arguments
        (Some(id), _, _) => Ok(id),

        // PostgreSQL with a working RETURNING statement
        (_, Ok(row), _) => Ok(GraphqlId::try_from(row.into_single().unwrap()).unwrap()),

        // We have an auto-incremented id that we got from MySQL or SQLite
        (_, _, Some(num)) => Ok(GraphqlId::from(num)),

        // Damn...
        (_, _, _) => panic!("Could not figure out an ID in create"),
    }
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

    Ok(ids)
}

pub async fn delete_records(conn: &dyn QueryExt, model: &ModelRef, where_: Filter) -> crate::Result<usize> {
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
    conn.query(query).await?;

    Ok(())
}

pub async fn disconnect(
    conn: &dyn QueryExt,
    field: &RelationFieldRef,
    parent_id: &GraphqlId,
    child_ids: &[GraphqlId],
) -> crate::Result<()> {
    let query = write::delete_relation_table_records(field, parent_id, child_ids);
    conn.query(query).await?;

    Ok(())
}
