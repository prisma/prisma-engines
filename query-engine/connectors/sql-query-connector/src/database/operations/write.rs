use super::update::*;
use crate::column_metadata;
use crate::filter::FilterBuilder;
use crate::row::ToSqlRow;
use crate::{
    error::SqlError, model_extensions::*, query_builder::write, sql_trace::SqlTraceComment, Context, QueryExt,
    Queryable,
};
use connector_interface::*;
use itertools::Itertools;
use prisma_models::*;
use quaint::{
    error::ErrorKind,
    prelude::{native_uuid, uuid_to_bin, uuid_to_bin_swapped, Aliasable, Select, SqlFamily},
};
use std::{
    collections::{HashMap, HashSet},
    ops::Deref,
    usize,
};
use user_facing_errors::query_engine::DatabaseConstraint;

#[cfg(target_arch = "wasm32")]
macro_rules! trace {
    (target: $target:expr, $($arg:tt)+) => {{
        // No-op in WebAssembly
    }};
    ($($arg:tt)+) => {{
        // No-op in WebAssembly
    }};
}

#[cfg(not(target_arch = "wasm32"))]
macro_rules! trace {
    (target: $target:expr, $($arg:tt)+) => {
        tracing::log::trace!(target: $target, $($arg)+);
    };
    ($($arg:tt)+) => {
        tracing::log::trace!($($arg)+);
    };
}

async fn generate_id(
    conn: &dyn Queryable,
    id_field: &FieldSelection,
    args: &WriteArgs,
    ctx: &Context<'_>,
) -> crate::Result<Option<SelectionResult>> {
    // Go through all the values and generate a select statement with the correct MySQL function
    let (id_select, need_select) = id_field
        .selections()
        .filter_map(|field| match field {
            SelectedField::Scalar(sf) if sf.default_value().is_some() && !args.has_arg_for(sf.db_name()) => sf
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
        let pk_select = id_select.add_trace_id(ctx.trace_id);
        let pk_result = conn.query(pk_select.into()).await?;
        let result = try_convert(&(id_field.into()), pk_result)?;

        Ok(Some(result))
    } else {
        Ok(None)
    }
}

/// Create a single record to the database defined in `conn`, resulting into a
/// `RecordProjection` as an identifier pointing to the just-created record.
pub(crate) async fn create_record(
    conn: &dyn Queryable,
    sql_family: &SqlFamily,
    model: &Model,
    mut args: WriteArgs,
    selected_fields: FieldSelection,
    ctx: &Context<'_>,
) -> crate::Result<SingleRecord> {
    let id_field: FieldSelection = model.primary_identifier();

    let returned_id = if *sql_family == SqlFamily::Mysql {
        generate_id(conn, &id_field, &args, ctx)
            .await?
            .or_else(|| args.as_selection_result(ModelProjection::from(id_field)))
    } else {
        args.as_selection_result(ModelProjection::from(id_field))
    };

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

    let insert = write::create_record(model, args, &ModelProjection::from(&selected_fields), ctx);

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
        // with a working RETURNING statement
        (_, n, _) if n > 0 => {
            let row = result_set.into_single()?;
            let field_names: Vec<_> = selected_fields.db_names().collect();
            let idents = ModelProjection::from(&selected_fields).type_identifiers_with_arities();
            let meta = column_metadata::create(&field_names, &idents);
            let sql_row = row.to_sql_row(&meta)?;
            let record = Record::from(sql_row);

            Ok(SingleRecord { record, field_names })
        }

        // All values provided in the write args
        (Some(identifier), _, _) if !identifier.misses_autogen_value() => {
            let field_names = identifier.db_names().map(ToOwned::to_owned).collect();
            let record = Record::from(identifier);

            Ok(SingleRecord { record, field_names })
        }

        // We have an auto-incremented id that we got from MySQL or SQLite
        (Some(mut identifier), _, Some(num)) if identifier.misses_autogen_value() => {
            identifier.add_autogen_value(num as i64);

            let field_names = identifier.db_names().map(ToOwned::to_owned).collect();
            let record = Record::from(identifier);

            Ok(SingleRecord { record, field_names })
        }

        (_, _, _) => panic!("Could not figure out an ID in create"),
    }
}

pub(crate) async fn create_records(
    conn: &dyn Queryable,
    model: &Model,
    args: Vec<WriteArgs>,
    skip_duplicates: bool,
    ctx: &Context<'_>,
) -> crate::Result<usize> {
    if args.is_empty() {
        return Ok(0);
    }

    // Compute the set of fields affected by the createMany.
    let mut fields = HashSet::new();
    args.iter().for_each(|arg| fields.extend(arg.keys()));

    #[allow(clippy::mutable_key_type)]
    let affected_fields: HashSet<ScalarFieldRef> = fields
        .into_iter()
        .map(|dsfn| model.fields().scalar().find(|sf| sf.db_name() == dsfn.deref()).unwrap())
        .collect();

    if affected_fields.is_empty() {
        // If no fields are to be inserted (everything is DEFAULT) we need to fall back to inserting default rows `args.len()` times.
        create_many_empty(conn, model, args.len(), skip_duplicates, ctx).await
    } else {
        create_many_nonempty(conn, model, args, skip_duplicates, affected_fields, ctx).await
    }
}

/// Standard create many records, requires `affected_fields` to be non-empty.
#[allow(clippy::mutable_key_type)]
async fn create_many_nonempty(
    conn: &dyn Queryable,
    model: &Model,
    args: Vec<WriteArgs>,
    skip_duplicates: bool,
    affected_fields: HashSet<ScalarFieldRef>,
    ctx: &Context<'_>,
) -> crate::Result<usize> {
    let batches = if let Some(max_params) = ctx.max_bind_values {
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

    let partitioned_batches = if let Some(max_rows) = ctx.max_rows {
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
    conn: &dyn Queryable,
    model: &Model,
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
    conn: &dyn Queryable,
    model: &Model,
    record_filter: RecordFilter,
    args: WriteArgs,
    selected_fields: Option<FieldSelection>,
    ctx: &Context<'_>,
) -> crate::Result<Option<SingleRecord>> {
    if let Some(selected_fields) = selected_fields {
        update_one_with_selection(conn, model, record_filter, args, selected_fields, ctx).await
    } else {
        update_one_without_selection(conn, model, record_filter, args, ctx).await
    }
}

/// Update multiple records in a database defined in `conn` and the records
/// defined in `args`, and returning the number of updates
/// This works via two ways, when there are ids in record_filter.selectors, it uses that to update
/// Otherwise it used the passed down arguments to update.
pub(crate) async fn update_records(
    conn: &dyn Queryable,
    model: &Model,
    record_filter: RecordFilter,
    args: WriteArgs,
    ctx: &Context<'_>,
) -> crate::Result<usize> {
    if args.args.is_empty() {
        return Ok(0);
    }

    if record_filter.has_selectors() {
        let (count, _) = update_many_from_ids_and_filter(conn, model, record_filter, args, ctx).await?;

        Ok(count)
    } else {
        let count = update_many_from_filter(conn, model, record_filter, args, ctx).await?;

        Ok(count)
    }
}

/// Delete multiple records in `conn`, defined in the `Filter`. Result is the number of items deleted.
pub(crate) async fn delete_records(
    conn: &dyn Queryable,
    model: &Model,
    record_filter: RecordFilter,
    ctx: &Context<'_>,
) -> crate::Result<usize> {
    let filter_condition = FilterBuilder::without_top_level_joins().visit_filter(record_filter.clone().filter, ctx);

    // If we have selectors, then we must chunk the mutation into multiple if necessary and add the ids to the filter.
    let row_count = if record_filter.has_selectors() {
        let ids: Vec<_> = record_filter.selectors.as_ref().unwrap().iter().collect();
        let mut row_count = 0;

        for delete in write::delete_many_from_ids_and_filter(model, ids.as_slice(), filter_condition, ctx) {
            row_count += conn.execute(delete).await?;
        }

        row_count
    } else {
        conn.execute(write::delete_many_from_filter(model, filter_condition, ctx))
            .await?
    };

    Ok(row_count as usize)
}

/// Connect relations defined in `child_ids` to a parent defined in `parent_id`.
/// The relation information is in the `RelationFieldRef`.
pub(crate) async fn m2m_connect(
    conn: &dyn Queryable,
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
    conn: &dyn Queryable,
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
    conn: &dyn Queryable,
    features: psl::PreviewFeatures,
    inputs: HashMap<String, PrismaValue>,
) -> crate::Result<usize> {
    let value = conn.raw_count(inputs, features).await?;

    Ok(value)
}

/// Execute a plain SQL query with the given parameters, returning the answer as
/// a JSON `Value`.
pub(crate) async fn query_raw(
    conn: &dyn Queryable,
    inputs: HashMap<String, PrismaValue>,
) -> crate::Result<serde_json::Value> {
    Ok(conn.raw_json(inputs).await?)
}
