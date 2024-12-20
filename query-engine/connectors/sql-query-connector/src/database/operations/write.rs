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
use quaint::ast::{Insert, Query};
use quaint::{
    error::ErrorKind,
    prelude::{native_uuid, uuid_to_bin, uuid_to_bin_swapped, Aliasable, Select, SqlFamily},
};
use query_structure::*;
use std::borrow::Cow;
use std::{
    collections::{HashMap, HashSet},
    ops::Deref,
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
        let pk_select = id_select.add_traceparent(ctx.traceparent);
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

    let returned_id = if sql_family.is_mysql() {
        generate_id(conn, &id_field, &args, ctx)
            .await?
            .or_else(|| args.as_selection_result(ModelProjection::from(id_field)))
    } else {
        args.as_selection_result(ModelProjection::from(id_field))
    };

    let args = match returned_id {
        Some(ref pk) if sql_family.is_mysql() => {
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
            let field_names = identifier.db_names().map(Cow::into_owned).collect();
            let record = Record::from(identifier);

            Ok(SingleRecord { record, field_names })
        }

        // We have an auto-incremented id that we got from MySQL or SQLite
        (Some(mut identifier), _, Some(num)) if identifier.misses_autogen_value() => {
            identifier.add_autogen_value(num as i64);

            let field_names = identifier.db_names().map(Cow::into_owned).collect();
            let record = Record::from(identifier);

            Ok(SingleRecord { record, field_names })
        }

        (_, _, _) => panic!("Could not figure out an ID in create"),
    }
}

/// Returns a set of fields that are used in the arguments for the create operation.
fn collect_affected_fields(args: &[WriteArgs], model: &Model) -> HashSet<ScalarFieldRef> {
    let mut fields = HashSet::new();
    args.iter().for_each(|arg| fields.extend(arg.keys()));

    fields
        .into_iter()
        .map(|dsfn| model.fields().scalar().find(|sf| sf.db_name() == dsfn.deref()).unwrap())
        .collect()
}

/// Generates a list of insert statements to execute. If `selected_fields` is set, insert statements
/// will return the specified columns of inserted rows.
fn generate_insert_statements(
    model: &Model,
    args: Vec<WriteArgs>,
    skip_duplicates: bool,
    selected_fields: Option<&ModelProjection>,
    ctx: &Context<'_>,
) -> Vec<Insert<'static>> {
    let affected_fields = collect_affected_fields(&args, model);

    if affected_fields.is_empty() {
        args.into_iter()
            .map(|_| write::create_records_empty(model, skip_duplicates, selected_fields, ctx))
            .collect()
    } else {
        let partitioned_batches = partition_into_batches(args, ctx);
        trace!("Total of {} batches to be executed.", partitioned_batches.len());
        trace!(
            "Batch sizes: {:?}",
            partitioned_batches.iter().map(|b| b.len()).collect_vec()
        );

        partitioned_batches
            .into_iter()
            .map(|batch| {
                write::create_records_nonempty(model, batch, skip_duplicates, &affected_fields, selected_fields, ctx)
            })
            .collect()
    }
}

/// Inserts records specified as a list of `WriteArgs`. Returns number of inserted records.
pub(crate) async fn create_records_count(
    conn: &dyn Queryable,
    model: &Model,
    args: Vec<WriteArgs>,
    skip_duplicates: bool,
    ctx: &Context<'_>,
) -> crate::Result<usize> {
    let inserts = generate_insert_statements(model, args, skip_duplicates, None, ctx);
    let mut count = 0;
    for insert in inserts {
        count += conn.execute(insert.into()).await?;
    }

    Ok(count as usize)
}

/// Inserts records specified as a list of `WriteArgs`. Returns values of fields specified in
/// `selected_fields` for all inserted rows.
pub(crate) async fn create_records_returning(
    conn: &dyn Queryable,
    model: &Model,
    args: Vec<WriteArgs>,
    skip_duplicates: bool,
    selected_fields: FieldSelection,
    ctx: &Context<'_>,
) -> crate::Result<ManyRecords> {
    let field_names: Vec<String> = selected_fields.db_names().collect();
    let idents = selected_fields.type_identifiers_with_arities();
    let meta = column_metadata::create(&field_names, &idents);
    let mut records = ManyRecords::new(field_names.clone());
    let inserts = generate_insert_statements(model, args, skip_duplicates, Some(&selected_fields.into()), ctx);

    for insert in inserts {
        let result_set = conn.query(insert.into()).await?;

        for result_row in result_set {
            let sql_row = result_row.to_sql_row(&meta)?;
            let record = Record::from(sql_row);

            records.push(record);
        }
    }

    Ok(records)
}

/// Partitions data into batches, respecting `max_bind_values` and `max_insert_rows` settings from
/// the `Context`.
fn partition_into_batches(args: Vec<WriteArgs>, ctx: &Context<'_>) -> Vec<Vec<WriteArgs>> {
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

    if let Some(max_rows) = ctx.max_insert_rows {
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
    }
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

async fn generate_updates(
    conn: &dyn Queryable,
    model: &Model,
    record_filter: RecordFilter,
    args: WriteArgs,
    selected_fields: Option<&ModelProjection>,
    ctx: &Context<'_>,
) -> crate::Result<Vec<Query<'static>>> {
    if record_filter.has_selectors() {
        let (updates, _) =
            update_many_from_ids_and_filter(conn, model, record_filter, args, selected_fields, ctx).await?;
        Ok(updates)
    } else {
        Ok(vec![
            update_many_from_filter(model, record_filter, args, selected_fields, ctx).await?,
        ])
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

    let mut count = 0;
    for update in generate_updates(conn, model, record_filter, args, None, ctx).await? {
        count += conn.execute(update).await?;
    }
    Ok(count as usize)
}

/// Update records according to `WriteArgs`. Returns values of fields specified in
/// `selected_fields` for all updated rows.
pub(crate) async fn update_records_returning(
    conn: &dyn Queryable,
    model: &Model,
    record_filter: RecordFilter,
    args: WriteArgs,
    selected_fields: FieldSelection,
    ctx: &Context<'_>,
) -> crate::Result<ManyRecords> {
    let field_names: Vec<String> = selected_fields.db_names().collect();
    let idents = selected_fields.type_identifiers_with_arities();
    let meta = column_metadata::create(&field_names, &idents);
    let mut records = ManyRecords::new(field_names.clone());

    let updates = generate_updates(conn, model, record_filter, args, Some(&selected_fields.into()), ctx).await?;

    for update in updates {
        let result_set = conn.query(update).await?;

        for result_row in result_set {
            let sql_row = result_row.to_sql_row(&meta)?;
            let record = Record::from(sql_row);

            records.push(record);
        }
    }

    Ok(records)
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

pub(crate) async fn delete_record(
    conn: &dyn Queryable,
    model: &Model,
    record_filter: RecordFilter,
    selected_fields: FieldSelection,
    ctx: &Context<'_>,
) -> crate::Result<SingleRecord> {
    // We explicitly checked in the query builder that there are no nested mutation
    // in combination with this operation.
    debug_assert!(!record_filter.has_selectors());

    let filter = FilterBuilder::without_top_level_joins().visit_filter(record_filter.filter, ctx);
    let selected_fields: ModelProjection = selected_fields.into();

    let result_set = conn
        .query(write::delete_returning(model, filter, &selected_fields, ctx))
        .await?;

    let mut result_iter = result_set.into_iter();
    let result_row = result_iter.next().ok_or(SqlError::RecordDoesNotExist {
        cause: "Record to delete does not exist.".to_owned(),
    })?;
    debug_assert!(result_iter.next().is_none(), "Filter returned more than one row. This is a bug because we must always require `id` in filters for `deleteOne` mutations");

    let field_db_names: Vec<_> = selected_fields.db_names().collect();
    let types_and_arities = selected_fields.type_identifiers_with_arities();
    let meta = column_metadata::create(&field_db_names, &types_and_arities);
    let sql_row = result_row.to_sql_row(&meta)?;

    let record = Record::from(sql_row);
    Ok(SingleRecord {
        record,
        field_names: field_db_names,
    })
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
pub(crate) async fn query_raw(conn: &dyn Queryable, inputs: HashMap<String, PrismaValue>) -> crate::Result<RawJson> {
    Ok(conn.raw_json(inputs).await?)
}
