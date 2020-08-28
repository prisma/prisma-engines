use crate::{error::SqlError, query_builder::write, QueryExt};
use connector_interface::*;
use prisma_models::*;
use prisma_value::PrismaValue;
use quaint::error::ErrorKind;
use std::{collections::HashMap, convert::TryFrom};
use user_facing_errors::query_engine::DatabaseConstraint;

/// Create a single record to the database defined in `conn`, resulting into a
/// `RecordProjection` as an identifier pointing to the just-created record.
pub async fn create_record(conn: &dyn QueryExt, model: &ModelRef, args: WriteArgs) -> crate::Result<RecordProjection> {
    let (insert, returned_id) = write::create_record(model, args);

    let result_set = match conn.insert(insert).await {
        Ok(id) => id,
        Err(e) => match e.kind() {
            ErrorKind::UniqueConstraintViolation { constraint } => match constraint {
                quaint::error::DatabaseConstraint::Index(name) => {
                    let constraint = DatabaseConstraint::Index(name.clone());
                    return Err(SqlError::UniqueConstraintViolation { constraint });
                }
                quaint::error::DatabaseConstraint::Fields(fields) => {
                    let constraint = DatabaseConstraint::Fields(fields.clone());
                    return Err(SqlError::UniqueConstraintViolation { constraint });
                }
                quaint::error::DatabaseConstraint::ForeignKey => {
                    let constraint = DatabaseConstraint::ForeignKey;
                    return Err(SqlError::UniqueConstraintViolation { constraint });
                }
            },
            ErrorKind::NullConstraintViolation { constraint } => match constraint {
                quaint::error::DatabaseConstraint::Index(name) => {
                    let constraint = DatabaseConstraint::Index(name.clone());
                    return Err(SqlError::NullConstraintViolation { constraint });
                }
                quaint::error::DatabaseConstraint::Fields(fields) => {
                    let constraint = DatabaseConstraint::Fields(fields.clone());
                    return Err(SqlError::NullConstraintViolation { constraint });
                }
                quaint::error::DatabaseConstraint::ForeignKey => {
                    let constraint = DatabaseConstraint::ForeignKey;
                    return Err(SqlError::UniqueConstraintViolation { constraint });
                }
            },
            _ => return Err(SqlError::from(e)),
        },
    };

    match (returned_id, result_set.len(), result_set.last_insert_id()) {
        // All values provided in the write arrghs
        (Some(identifier), _, _) if !identifier.misses_autogen_value() => Ok(identifier),

        // PostgreSQL with a working RETURNING statement
        (_, n, _) if n > 0 => Ok(RecordProjection::try_from((&model.primary_identifier(), result_set))?),

        // We have an auto-incremented id that we got from MySQL or SQLite
        (Some(mut identifier), _, Some(num)) if identifier.misses_autogen_value() => {
            identifier.add_autogen_value(num as i64);
            Ok(identifier)
        }

        (_, _, _) => panic!("Could not figure out an ID in create"),
    }
}

/// Update multiple records in a database defined in `conn` and the records
/// defined in `args`, resulting the identifiers that were modified in the
/// operation.
pub async fn update_records(
    conn: &dyn QueryExt,
    model: &ModelRef,
    record_filter: RecordFilter,
    args: WriteArgs,
) -> crate::Result<Vec<RecordProjection>> {
    let ids = conn.filter_selectors(model, record_filter).await?;
    let id_args = pick_args(&model.primary_identifier(), &args);

    if ids.len() == 0 {
        return Ok(vec![]);
    }

    let updates = {
        let ids: Vec<&RecordProjection> = ids.iter().map(|id| &*id).collect();
        write::update_many(model, ids.as_slice(), args)?
    };

    for update in updates {
        conn.query(update).await?;
    }

    Ok(merge_write_args(ids, id_args))
}

/// Delete multiple records in `conn`, defined in the `Filter`. Result is the number of items deleted.
pub async fn delete_records(
    conn: &dyn QueryExt,
    model: &ModelRef,
    record_filter: RecordFilter,
) -> crate::Result<usize> {
    let ids = conn.filter_selectors(model, record_filter).await?;
    let ids: Vec<&RecordProjection> = ids.iter().map(|id| &*id).collect();
    let count = ids.len();

    if count == 0 {
        return Ok(count);
    }

    for delete in write::delete_many(model, ids.as_slice()) {
        conn.query(delete).await?;
    }

    Ok(count)
}

/// Connect relations defined in `child_ids` to a parent defined in `parent_id`.
/// The relation information is in the `RelationFieldRef`.
pub async fn connect(
    conn: &dyn QueryExt,
    field: &RelationFieldRef,
    parent_id: &RecordProjection,
    child_ids: &[RecordProjection],
) -> crate::Result<()> {
    let query = write::create_relation_table_records(field, parent_id, child_ids);
    conn.query(query).await?;

    Ok(())
}

/// Disconnect relations defined in `child_ids` to a parent defined in `parent_id`.
/// The relation information is in the `RelationFieldRef`.
pub async fn disconnect(
    conn: &dyn QueryExt,
    field: &RelationFieldRef,
    parent_id: &RecordProjection,
    child_ids: &[RecordProjection],
) -> crate::Result<()> {
    let query = write::delete_relation_table_records(field, parent_id, child_ids);
    conn.delete(query).await?;

    Ok(())
}

/// Execute a plain SQL query with the given parameters, returning the number of
/// affected rows.
pub async fn execute_raw(conn: &dyn QueryExt, query: String, parameters: Vec<PrismaValue>) -> crate::Result<usize> {
    let value = conn.raw_count(query, parameters).await?;
    Ok(value)
}

/// Execute a plain SQL query with the given parameters, returning the answer as
/// a JSON `Value`.
pub async fn query_raw(
    conn: &dyn QueryExt,
    query: String,
    parameters: Vec<PrismaValue>,
) -> crate::Result<serde_json::Value> {
    let value = conn.raw_json(query, parameters).await?;
    Ok(value)
}

/// Picks all arguments out of `args` that are updating a value for a field
/// contained in `projection`, as those need to be merged into the records later on.
fn pick_args(projection: &ModelProjection, args: &WriteArgs) -> WriteArgs {
    let pairs: Vec<_> = projection
        .scalar_fields()
        .into_iter()
        .filter_map(|field| {
            args.get_field_value(field.db_name())
                .map(|v| (DatasourceFieldName::from(&field), v.clone()))
        })
        .collect();

    WriteArgs::from(pairs)
}

/// Merges the incoming write argument values into the given, already loaded, ids. Overwrites existing values.
fn merge_write_args(loaded_ids: Vec<RecordProjection>, incoming_args: WriteArgs) -> Vec<RecordProjection> {
    if loaded_ids.is_empty() || incoming_args.is_empty() {
        return loaded_ids;
    }

    // Contains all positions that need to be updated with the given expression.
    let positions: HashMap<usize, &WriteExpression> = loaded_ids
        .first()
        .unwrap()
        .pairs
        .iter()
        .enumerate()
        .filter_map(|(i, (field, _))| incoming_args.get_field_value(field.db_name()).map(|val| (i, val)))
        .collect();

    loaded_ids
        .into_iter()
        .map(|mut id| {
            for (position, expr) in positions.iter() {
                let current_val = id.pairs[position.to_owned()].1.clone();
                id.pairs[position.to_owned()].1 = apply_expression(current_val, (*expr).clone());
            }

            id
        })
        .collect()
}

fn apply_expression(val: PrismaValue, expr: WriteExpression) -> PrismaValue {
    match expr {
        WriteExpression::Field(_) => unimplemented!(),
        WriteExpression::Value(pv) => pv,
        WriteExpression::Add(rhs) => val + rhs,
        WriteExpression::Substract(rhs) => val - rhs,
        WriteExpression::Multiply(rhs) => val * rhs,
        WriteExpression::Divide(rhs) => val / rhs,
    }
}
