use crate::{error::SqlError, query_builder::write, sql_info::SqlInfo, QueryExt};
use connector_interface::*;
use itertools::Itertools;
use prisma_models::*;
use prisma_value::PrismaValue;
use quaint::error::ErrorKind;
use std::{convert::TryFrom, usize};
use tracing::log::trace;
use user_facing_errors::query_engine::DatabaseConstraint;

/// Create a single record to the database defined in `conn`, resulting into a
/// `RecordProjection` as an identifier pointing to the just-created record.
#[tracing::instrument(skip(conn, model, args))]
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
                quaint::error::DatabaseConstraint::CannotParse => {
                    let constraint = DatabaseConstraint::CannotParse;
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
                quaint::error::DatabaseConstraint::CannotParse => {
                    let constraint = DatabaseConstraint::CannotParse;
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

#[tracing::instrument(skip(conn, sql_info, model, args, skip_duplicates))]
pub async fn create_records(
    conn: &dyn QueryExt,
    sql_info: SqlInfo,
    model: &ModelRef,
    args: Vec<WriteArgs>,
    skip_duplicates: bool,
) -> crate::Result<usize> {
    if args.is_empty() {
        return Ok(0);
    }

    let batches = if let Some(max_params) = sql_info.max_bind_values {
        // We need to split inserts if they are above a parameter threshold, as well as split based on number of rows.
        // -> Horizontal partitioning by row number, vertical by number of args.
        args.into_iter()
            .peekable()
            .batching(|iter| {
                let mut param_count: usize = 0;
                let mut batch = vec![];

                while param_count < max_params {
                    // If the param count _including_ the next item doens't exceed the limit,
                    // we continue filling up the current batch.
                    let proceed = match iter.peek() {
                        Some(next) => (param_count + next.len()) <= max_params,
                        None => break,
                    };

                    if proceed {
                        match iter.next() {
                            Some(next) => {
                                param_count += next.len();
                                batch.push(next)
                            }
                            None => break,
                        }
                    } else {
                        break;
                    }
                }

                if batch.is_empty() {
                    None
                } else {
                    Some(batch)
                }
            })
            .collect_vec()
    } else {
        vec![args]
    };

    let partitioned_batches = if let Some(max_rows) = sql_info.max_rows {
        let capacity = batches.len();
        batches
            .into_iter()
            .fold(Vec::with_capacity(capacity), |mut batches, next_batch| {
                if next_batch.len() > max_rows {
                    batches.extend(
                        next_batch
                            .into_iter()
                            .chunks(max_rows)
                            .into_iter()
                            .map(|chunk| chunk.into_iter().collect_vec()),
                    );
                } else {
                    batches.push(next_batch);
                }

                batches
            })
    } else {
        batches
    };

    trace!("Total of {} batches to be executed.", partitioned_batches.len());
    trace!(
        "Batch sizes: {:?}",
        partitioned_batches.iter().map(|b| b.len()).collect_vec()
    );

    let mut count = 0;
    for batch in partitioned_batches {
        let stmt = write::create_records(model, batch, skip_duplicates);
        count += conn.execute(stmt.into()).await?;
    }

    Ok(count as usize)
}

/// Update multiple records in a database defined in `conn` and the records
/// defined in `args`, resulting the identifiers that were modified in the
/// operation.
#[tracing::instrument(skip(conn, model, record_filter, args))]
pub async fn update_records(
    conn: &dyn QueryExt,
    model: &ModelRef,
    record_filter: RecordFilter,
    args: WriteArgs,
) -> crate::Result<Vec<RecordProjection>> {
    let ids = conn.filter_selectors(model, record_filter).await?;
    let id_args = pick_args(&model.primary_identifier(), &args);

    if ids.is_empty() {
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
#[tracing::instrument(skip(conn, model, record_filter))]
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
#[tracing::instrument(skip(conn, field, parent_id, child_ids))]
pub async fn m2m_connect(
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
#[tracing::instrument(skip(conn, field, parent_id, child_ids))]
pub async fn m2m_disconnect(
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
#[tracing::instrument(skip(conn, query, parameters))]
pub async fn execute_raw(conn: &dyn QueryExt, query: String, parameters: Vec<PrismaValue>) -> crate::Result<usize> {
    let value = conn.raw_count(query, parameters).await?;
    Ok(value)
}

/// Execute a plain SQL query with the given parameters, returning the answer as
/// a JSON `Value`.
#[tracing::instrument(skip(conn, query, parameters))]
pub async fn query_raw(
    conn: &dyn QueryExt,
    query: String,
    parameters: Vec<PrismaValue>,
) -> crate::Result<serde_json::Value> {
    let value = conn.raw_json(query, parameters).await?;
    Ok(value)
}
