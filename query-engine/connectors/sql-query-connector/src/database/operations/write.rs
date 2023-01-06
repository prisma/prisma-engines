use crate::filter_conversion::AliasedCondition;
use crate::query_builder::write::{build_update_and_set_query, chunk_update_with_ids};
use crate::{
    error::SqlError, model_extensions::*, query_builder::write, sql_info::SqlInfo, sql_trace::SqlTraceComment, Context,
    QueryExt,
};
use connector_interface::*;
use itertools::Itertools;
use prisma_models::*;
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
    args: &WriteArgs,
    ctx: &Context<'_>,
) -> crate::Result<Option<SelectionResult>> {
    // Go through all the values and generate a select statement with the correct MySQL function
    let (pk_select, need_select) = primary_key
        .selections()
        .filter_map(|field| match field {
            SelectedField::Scalar(x) if x.default_value().is_some() && !args.has_arg_for(x.db_name()) => x
                .default_value()
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
        let pk_select = pk_select.add_trace_id(ctx.trace_id);
        let pk_result = conn.query(pk_select.into()).await?;
        let result = try_convert(&(primary_key.into()), pk_result)?;

        Ok(Some(result))
    } else {
        Ok(None)
    }
}

/// Create a single record to the database defined in `conn`, resulting into a
/// `RecordProjection` as an identifier pointing to the just-created record.
pub(crate) async fn create_record(
    conn: &dyn QueryExt,
    sql_family: &SqlFamily,
    model: &ModelRef,
    mut args: WriteArgs,
    ctx: &Context<'_>,
) -> crate::Result<SelectionResult> {
    let pk = model.primary_identifier();

    let returned_id = if *sql_family == SqlFamily::Mysql {
        generate_id(conn, &pk, &args, ctx).await?
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

    let insert = write::create_record(model, args, ctx);

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

        // with a working RETURNING statement
        (_, n, _) if n > 0 => Ok(try_convert(&model.primary_identifier().into(), result_set)?),

        // We have an auto-incremented id that we got from MySQL or SQLite
        (Some(mut identifier), _, Some(num)) if identifier.misses_autogen_value() => {
            identifier.add_autogen_value(num as i64);
            Ok(identifier)
        }

        (_, _, _) => panic!("Could not figure out an ID in create"),
    }
}

pub(crate) async fn create_records(
    conn: &dyn QueryExt,
    sql_info: SqlInfo,
    model: &ModelRef,
    args: Vec<WriteArgs>,
    skip_duplicates: bool,
    ctx: &Context<'_>,
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
        create_many_empty(conn, model, args.len(), skip_duplicates, ctx).await
    } else {
        create_many_nonempty(conn, sql_info, model, args, skip_duplicates, affected_fields, ctx).await
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
    ctx: &Context<'_>,
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
        let stmt = write::create_records_nonempty(model, batch, skip_duplicates, &affected_fields, ctx);
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
    ctx: &Context<'_>,
) -> crate::Result<usize> {
    let stmt = write::create_records_empty(model, skip_duplicates, ctx);
    let mut count = 0;

    for _ in 0..num_records {
        count += conn.execute(stmt.clone().into()).await?;
    }

    Ok(count as usize)
}

/// Update one record in a database defined in `conn` and the records
/// defined in `args`, resulting the identifiers that were modified in the
/// operation.
pub(crate) async fn update_record(
    conn: &dyn QueryExt,
    model: &ModelRef,
    record_filter: RecordFilter,
    args: WriteArgs,
    ctx: &Context<'_>,
) -> crate::Result<Vec<SelectionResult>> {
    let id_args = pick_args(&model.primary_identifier().into(), &args);

    // This is to match the behaviour expected but it seems a bit strange to me
    // This comes across as if the update happened even if it didn't
    if args.args.is_empty() {
        let ids: Vec<SelectionResult> = conn.filter_selectors(model, record_filter.clone(), ctx).await?;

        return Ok(ids);
    }

    let (_, ids) = update_records_from_ids_and_filter(conn, model, record_filter, args, ctx).await?;

    Ok(merge_write_args(ids, id_args))
}

// Generates a query like this:
//  UPDATE "public"."User" SET "name" = $1 WHERE "public"."User"."id" IN ($2,$3,$4,$5,$6,$7,$8,$9,$10,$11) AND "public"."User"."age" > $1
async fn update_records_from_ids_and_filter(
    conn: &dyn QueryExt,
    model: &ModelRef,
    record_filter: RecordFilter,
    args: WriteArgs,
    ctx: &Context<'_>,
) -> crate::Result<(usize, Vec<SelectionResult>)> {
    let filter_condition = record_filter.clone().filter.aliased_condition_from(None, false, ctx);
    let ids: Vec<SelectionResult> = conn.filter_selectors(model, record_filter, ctx).await?;

    if ids.is_empty() {
        return Ok((0, Vec::new()));
    }

    let update = build_update_and_set_query(model, args, ctx);

    let updates = {
        let ids: Vec<&SelectionResult> = ids.iter().collect();
        chunk_update_with_ids(update, model, &ids, filter_condition, ctx)?
    };

    let mut count = 0;
    for update in updates {
        let update_count = conn.execute(update).await?;

        count += update_count;
    }

    Ok((count as usize, ids))
}

// Generates a query like this:
//  UPDATE "public"."User" SET "name" = $1 WHERE "public"."User"."age" > $1
async fn update_records_from_filter(
    conn: &dyn QueryExt,
    model: &ModelRef,
    record_filter: RecordFilter,
    args: WriteArgs,
    ctx: &Context<'_>,
) -> crate::Result<usize> {
    let update = build_update_and_set_query(model, args, ctx);
    let filter_condition = record_filter.clone().filter.aliased_condition_from(None, false, ctx);

    let update = update.so_that(filter_condition);
    let count = conn.execute(update.into()).await?;

    Ok(count as usize)
}

/// Update multiple records in a database defined in `conn` and the records
/// defined in `args`, and returning the number of updates
/// This works via two ways, when there are ids in record_filter.selectors, it uses that to update
/// Otherwise it used the passed down arguments to update.
pub(crate) async fn update_records(
    conn: &dyn QueryExt,
    model: &ModelRef,
    record_filter: RecordFilter,
    args: WriteArgs,
    ctx: &Context<'_>,
) -> crate::Result<usize> {
    if args.args.is_empty() {
        return Ok(0);
    }

    if record_filter.has_selectors() {
        let (count, _) = update_records_from_ids_and_filter(conn, model, record_filter, args, ctx).await?;
        Ok(count)
    } else {
        update_records_from_filter(conn, model, record_filter, args, ctx).await
    }
}

/// Delete multiple records in `conn`, defined in the `Filter`. Result is the number of items deleted.
pub(crate) async fn delete_records(
    conn: &dyn QueryExt,
    model: &ModelRef,
    record_filter: RecordFilter,
    ctx: &Context<'_>,
) -> crate::Result<usize> {
    let filter_condition = record_filter.clone().filter.aliased_condition_from(None, false, ctx);
    let ids = conn.filter_selectors(model, record_filter, ctx).await?;
    let ids: Vec<&SelectionResult> = ids.iter().collect();
    let count = ids.len();

    if count == 0 {
        return Ok(count);
    }

    let mut row_count = 0;
    for delete in write::delete_many(model, ids.as_slice(), filter_condition, ctx) {
        row_count += conn.execute(delete).await?;
    }

    match usize::try_from(row_count) {
        Ok(row_count) => Ok(row_count),
        Err(_) => Ok(count),
    }
}

/// Connect relations defined in `child_ids` to a parent defined in `parent_id`.
/// The relation information is in the `RelationFieldRef`.
pub(crate) async fn m2m_connect(
    conn: &dyn QueryExt,
    field: &RelationFieldRef,
    parent_id: &SelectionResult,
    child_ids: &[SelectionResult],
    ctx: &Context<'_>,
) -> crate::Result<()> {
    let query = write::create_relation_table_records(field, parent_id, child_ids, ctx);
    conn.query(query).await?;

    Ok(())
}

/// Disconnect relations defined in `child_ids` to a parent defined in `parent_id`.
/// The relation information is in the `RelationFieldRef`.
pub(crate) async fn m2m_disconnect(
    conn: &dyn QueryExt,
    field: &RelationFieldRef,
    parent_id: &SelectionResult,
    child_ids: &[SelectionResult],
    ctx: &Context<'_>,
) -> crate::Result<()> {
    let query = write::delete_relation_table_records(field, parent_id, child_ids, ctx);
    conn.delete(query).await?;

    Ok(())
}

/// Execute a plain SQL query with the given parameters, returning the number of
/// affected rows.
pub(crate) async fn execute_raw(
    conn: &dyn QueryExt,
    features: psl::PreviewFeatures,
    inputs: HashMap<String, PrismaValue>,
) -> crate::Result<usize> {
    let value = conn.raw_count(inputs, features).await?;

    Ok(value)
}

/// Execute a plain SQL query with the given parameters, returning the answer as
/// a JSON `Value`.
pub(crate) async fn query_raw(
    conn: &dyn QueryExt,
    sql_info: SqlInfo,
    features: psl::PreviewFeatures,
    inputs: HashMap<String, PrismaValue>,
) -> crate::Result<serde_json::Value> {
    let value = conn.raw_json(sql_info, features, inputs).await?;

    Ok(value)
}
