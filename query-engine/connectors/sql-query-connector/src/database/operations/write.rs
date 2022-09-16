use crate::filter_conversion::AliasedCondition;
use crate::sql_trace::SqlTraceComment;
use crate::{error::SqlError, model_extensions::*, query_builder::write, sql_info::SqlInfo, QueryExt};
use connector_interface::*;
use itertools::Itertools;
use prisma_models::*;
use psl::common::preview_features::PreviewFeature;
use psl::dml::prisma_value::PrismaValue;
use quaint::{
    error::ErrorKind,
    prelude::{native_uuid, uuid_to_bin, uuid_to_bin_swapped, Aliasable, Select, SqlFamily},
};
use std::{
    collections::{HashMap, HashSet},
    ops::Deref,
    usize,
};
use tracing::log::trace;
use user_facing_errors::query_engine::DatabaseConstraint;

async fn generate_id(
    conn: &dyn QueryExt,
    primary_key: &FieldSelection,
    trace_id: Option<String>,
    args: &WriteArgs,
) -> crate::Result<Option<SelectionResult>> {
    // Go through all the values and generate a select statement with the correct MySQL function
    let (pk_select, need_select) = primary_key
        .selections()
        .filter_map(|field| match field {
            SelectedField::Scalar(x) if x.default_value.is_some() && !args.has_arg_for(x.db_name()) => x
                .default_value
                .clone()
                .unwrap()
                .to_dbgenerated_func()
                .map(|func| (field.db_name().to_string(), func)),
            _ => None,
        })
        .fold((Select::default(), false), |(query, generated), value| {
            let alias = value.0;
            let func = value.1.to_lowercase().replace(' ', "");

            match func.as_str() {
                "(uuid())" => (query.value(native_uuid().alias(alias)), true),
                "(uuid_to_bin(uuid()))" | "(uuid_to_bin(uuid(),0))" => (query.value(uuid_to_bin().alias(alias)), true),
                "(uuid_to_bin(uuid(),1))" => (query.value(uuid_to_bin_swapped().alias(alias)), true),
                _ => (query, generated),
            }
        });

    // db generate values only if needed
    if need_select {
        let pk_select = pk_select.add_trace_id(trace_id);
        let pk_result = conn.query(pk_select.into()).await?;
        let result = try_convert(&(primary_key.into()), pk_result)?;

        Ok(Some(result))
    } else {
        Ok(None)
    }
}

/// Create a single record to the database defined in `conn`, resulting into a
/// `RecordProjection` as an identifier pointing to the just-created record.
pub async fn create_record(
    conn: &dyn QueryExt,
    sql_family: &SqlFamily,
    model: &ModelRef,
    mut args: WriteArgs,
    trace_id: Option<String>,
) -> crate::Result<SelectionResult> {
    let pk = model.primary_identifier();

    let returned_id = if *sql_family == SqlFamily::Mysql {
        generate_id(conn, &pk, trace_id.clone(), &args).await?
    } else {
        args.as_record_projection(pk.clone().into())
    };

    let returned_id = returned_id.or_else(|| args.as_record_projection(pk.clone().into()));

    let args = match returned_id {
        Some(ref pk) if *sql_family == SqlFamily::Mysql => {
            for (field, value) in pk.pairs.iter() {
                let field = DatasourceFieldName(field.db_name().into());
                let value = WriteOperation::scalar_set(value.clone());
                args.insert(field, value)
            }
            args
        }
        _ => args,
    };

    let insert = write::create_record(model, args, trace_id);

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
                    return Err(SqlError::NullConstraintViolation { constraint });
                }
                quaint::error::DatabaseConstraint::CannotParse => {
                    let constraint = DatabaseConstraint::CannotParse;
                    return Err(SqlError::NullConstraintViolation { constraint });
                }
            },
            _ => return Err(SqlError::from(e)),
        },
    };

    match (returned_id, result_set.len(), result_set.last_insert_id()) {
        // All values provided in the write arrghs
        (Some(identifier), _, _) if !identifier.misses_autogen_value() => Ok(identifier),

        // PostgreSQL with a working RETURNING statement
        (_, n, _) if n > 0 => Ok(try_convert(&model.primary_identifier().into(), result_set)?),

        // We have an auto-incremented id that we got from MySQL or SQLite
        (Some(mut identifier), _, Some(num)) if identifier.misses_autogen_value() => {
            identifier.add_autogen_value(num as i64);
            Ok(identifier)
        }

        (_, _, _) => panic!("Could not figure out an ID in create"),
    }
}

pub async fn create_records(
    conn: &dyn QueryExt,
    sql_info: SqlInfo,
    model: &ModelRef,
    args: Vec<WriteArgs>,
    skip_duplicates: bool,
    trace_id: Option<String>,
) -> crate::Result<usize> {
    if args.is_empty() {
        return Ok(0);
    }

    // Compute the set of fields affected by the createMany.
    let mut fields = HashSet::new();
    args.iter().for_each(|arg| fields.extend(arg.keys().into_iter()));

    #[allow(clippy::mutable_key_type)]
    let affected_fields: HashSet<ScalarFieldRef> = fields
        .into_iter()
        .map(|dsfn| {
            model
                .fields()
                .scalar()
                .into_iter()
                .find(|sf| sf.db_name() == dsfn.deref())
                .unwrap()
        })
        .collect();

    if affected_fields.is_empty() {
        // If no fields are to be inserted (everything is DEFAULT) we need to fall back to inserting default rows `args.len()` times.
        create_many_empty(conn, model, args.len(), skip_duplicates, trace_id).await
    } else {
        create_many_nonempty(conn, sql_info, model, args, skip_duplicates, affected_fields, trace_id).await
    }
}

/// Standard create many records, requires `affected_fields` to be non-empty.
#[allow(clippy::mutable_key_type)]
async fn create_many_nonempty(
    conn: &dyn QueryExt,
    sql_info: SqlInfo,
    model: &ModelRef,
    args: Vec<WriteArgs>,
    skip_duplicates: bool,
    affected_fields: HashSet<ScalarFieldRef>,
    trace_id: Option<String>,
) -> crate::Result<usize> {
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
        let stmt = write::create_records_nonempty(model, batch, skip_duplicates, &affected_fields, trace_id.clone());
        count += conn.execute(stmt.into()).await?;
    }

    Ok(count as usize)
}

/// Creates many empty (all default values) rows.
async fn create_many_empty(
    conn: &dyn QueryExt,
    model: &ModelRef,
    num_records: usize,
    skip_duplicates: bool,
    trace_id: Option<String>,
) -> crate::Result<usize> {
    let stmt = write::create_records_empty(model, skip_duplicates, trace_id);
    let mut count = 0;

    for _ in 0..num_records {
        count += conn.execute(stmt.clone().into()).await?;
    }

    Ok(count as usize)
}

/// Update one record in a database defined in `conn` and the records
/// defined in `args`, resulting the identifiers that were modified in the
/// operation.
pub async fn update_record(
    conn: &dyn QueryExt,
    model: &ModelRef,
    record_filter: RecordFilter,
    args: WriteArgs,
    trace_id: Option<String>,
) -> crate::Result<Vec<SelectionResult>> {
    let filter_condition = record_filter.clone().filter.aliased_condition_from(None, false);
    let ids = conn.filter_selectors(model, record_filter, trace_id.clone()).await?;
    let id_args = pick_args(&model.primary_identifier().into(), &args);

    if ids.is_empty() {
        return Ok(vec![]);
    }

    let updates = {
        let ids: Vec<&SelectionResult> = ids.iter().collect();
        write::update_many(model, ids.as_slice(), args, filter_condition, trace_id)?
    };

    for update in updates {
        conn.execute(update).await?;
    }

    Ok(merge_write_args(ids, id_args))
}

/// Update multiple records in a database defined in `conn` and the records
/// defined in `args`, and returning the number of updates
/// This function via two ways, when there are ids in record_filter.selectors, it uses that to update
/// Otherwise it used the passed down arguments to update.
/// Future clean up - we should split this into two functions, one that handles updates based on the selector
/// and another that does an update based on the WriteArgs
pub async fn update_records(
    conn: &dyn QueryExt,
    model: &ModelRef,
    record_filter: RecordFilter,
    args: WriteArgs,
    trace_id: Option<String>,
) -> crate::Result<usize> {
    let filter_condition = record_filter.clone().filter.aliased_condition_from(None, false);

    // If no where filter is supplied we need to get the ids for the update
    let ids: Vec<SelectionResult> = if record_filter.filter.is_empty() {
        let ids = conn.filter_selectors(model, record_filter, trace_id.clone()).await?;
        if ids.is_empty() {
            return Ok(0);
        }
        ids
    } else {
        Vec::new()
    };

    let updates = {
        let ids: Vec<&SelectionResult> = ids.iter().collect();
        write::update_many(model, ids.as_slice(), args, filter_condition, trace_id)?
    };

    let mut count = 0;
    for update in updates {
        let update_count = conn.execute(update).await?;

        count += update_count;
    }

    Ok(count as usize)
}

/// Delete multiple records in `conn`, defined in the `Filter`. Result is the number of items deleted.
pub async fn delete_records(
    conn: &dyn QueryExt,
    model: &ModelRef,
    record_filter: RecordFilter,
    trace_id: Option<String>,
) -> crate::Result<usize> {
    let filter_condition = record_filter.clone().filter.aliased_condition_from(None, false);
    let ids = conn.filter_selectors(model, record_filter, trace_id.clone()).await?;
    let ids: Vec<&SelectionResult> = ids.iter().collect();
    let count = ids.len();

    if count == 0 {
        return Ok(count);
    }

    let mut row_count = 0;
    for delete in write::delete_many(model, ids.as_slice(), filter_condition, trace_id) {
        row_count += conn.execute(delete).await?;
    }

    match usize::try_from(row_count) {
        Ok(row_count) => Ok(row_count),
        Err(_) => Ok(count),
    }
}

/// Connect relations defined in `child_ids` to a parent defined in `parent_id`.
/// The relation information is in the `RelationFieldRef`.
pub async fn m2m_connect(
    conn: &dyn QueryExt,
    field: &RelationFieldRef,
    parent_id: &SelectionResult,
    child_ids: &[SelectionResult],
) -> crate::Result<()> {
    let query = write::create_relation_table_records(field, parent_id, child_ids);
    conn.query(query).await?;

    Ok(())
}

/// Disconnect relations defined in `child_ids` to a parent defined in `parent_id`.
/// The relation information is in the `RelationFieldRef`.
pub async fn m2m_disconnect(
    conn: &dyn QueryExt,
    field: &RelationFieldRef,
    parent_id: &SelectionResult,
    child_ids: &[SelectionResult],
    trace_id: Option<String>,
) -> crate::Result<()> {
    let query = write::delete_relation_table_records(field, parent_id, child_ids, trace_id);
    conn.delete(query).await?;

    Ok(())
}

/// Execute a plain SQL query with the given parameters, returning the number of
/// affected rows.
pub async fn execute_raw(
    conn: &dyn QueryExt,
    features: &[PreviewFeature],
    inputs: HashMap<String, PrismaValue>,
) -> crate::Result<usize> {
    let value = conn.raw_count(inputs, features).await?;

    Ok(value)
}

/// Execute a plain SQL query with the given parameters, returning the answer as
/// a JSON `Value`.
pub async fn query_raw(
    conn: &dyn QueryExt,
    sql_info: SqlInfo,
    features: &[PreviewFeature],
    inputs: HashMap<String, PrismaValue>,
) -> crate::Result<serde_json::Value> {
    let value = conn.raw_json(sql_info, features, inputs).await?;

    Ok(value)
}
