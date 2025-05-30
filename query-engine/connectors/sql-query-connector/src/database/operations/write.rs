use super::update::*;
use crate::row::ToSqlRow;
use crate::value::to_prisma_value;
use crate::{error::SqlError, QueryExt, Queryable};
use itertools::Itertools;
use quaint::prelude::ResultSet;
use quaint::{
    error::ErrorKind,
    prelude::{Select, SqlFamily},
};
use query_structure::*;
use sql_query_builder::write::defaults_for_mysql_write_args;
use sql_query_builder::{column_metadata, write, Context, SelectionResultExt, SqlTraceComment};
use std::borrow::Cow;
use std::collections::HashMap;
use user_facing_errors::query_engine::DatabaseConstraint;

async fn generate_id(
    conn: &dyn Queryable,
    id_field: &FieldSelection,
    args: &WriteArgs,
    ctx: &Context<'_>,
) -> crate::Result<Option<SelectionResult>> {
    // Go through all the values and generate a select statement with the correct MySQL function
    let defaults = defaults_for_mysql_write_args(id_field, args)
        .map(|(_, val)| val)
        .collect_vec();

    // db generate values only if needed
    if !defaults.is_empty() {
        let mut id_select = Select::default();
        id_select.extend(defaults);

        let pk_select = id_select.add_traceparent(ctx.traceparent());
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

/// Inserts records specified as a list of `WriteArgs`. Returns number of inserted records.
pub(crate) async fn create_records_count(
    conn: &dyn Queryable,
    model: &Model,
    args: Vec<WriteArgs>,
    skip_duplicates: bool,
    ctx: &Context<'_>,
) -> crate::Result<usize> {
    let inserts = write::generate_insert_statements(model, args, skip_duplicates, None, ctx);
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
    let inserts = write::generate_insert_statements(model, args, skip_duplicates, Some(&selected_fields.into()), ctx);

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
    limit: Option<usize>,
    ctx: &Context<'_>,
) -> crate::Result<usize> {
    if args.args.is_empty() {
        return Ok(0);
    }

    let mut count = 0;
    for update in write::generate_update_statements(model, record_filter, args, None, limit, ctx) {
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
    limit: Option<usize>,
    ctx: &Context<'_>,
) -> crate::Result<ManyRecords> {
    let field_names: Vec<String> = selected_fields.db_names().collect();
    let idents = selected_fields.type_identifiers_with_arities();
    let meta = column_metadata::create(&field_names, &idents);
    let mut records = ManyRecords::new(field_names.clone());

    for update in
        write::generate_update_statements(model, record_filter, args, Some(&selected_fields.into()), limit, ctx)
    {
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
    limit: Option<usize>,
    ctx: &Context<'_>,
) -> crate::Result<usize> {
    let mut row_count = 0;
    let mut remaining_limit = limit;

    for delete in write::generate_delete_statements(model, record_filter, limit, ctx) {
        row_count += conn.execute(delete).await?;
        if let Some(old_remaining_limit) = remaining_limit {
            // u64 to usize cast here cannot 'overflow' as the number of rows was limited to MAX usize in the first place.
            let new_remaining_limit = old_remaining_limit - row_count as usize;
            if new_remaining_limit == 0 {
                break;
            }
            remaining_limit = Some(new_remaining_limit);
        }
    }

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

    let selected_fields: ModelProjection = selected_fields.into();

    let result_set = conn
        .query(write::delete_returning(
            model,
            record_filter.filter,
            &selected_fields,
            ctx,
        ))
        .await?;

    let mut result_iter = result_set.into_iter();
    let result_row = result_iter.next().ok_or(SqlError::RecordDoesNotExist {
        cause: "No record was found for a delete.".to_owned(),
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

fn try_convert(model_projection: &ModelProjection, result_set: ResultSet) -> crate::Result<SelectionResult> {
    let columns: Vec<String> = result_set.columns().iter().map(|c| c.to_string()).collect();
    let mut record_projection = SelectionResult::default();

    if let Some(row) = result_set.into_iter().next() {
        for (i, val) in row.into_iter().enumerate() {
            match model_projection.map_db_name(columns[i].as_str()) {
                Some(field) => {
                    record_projection.add((field, to_prisma_value(val)?));
                }
                None => {
                    return Err(SqlError::DomainError(DomainError::ScalarFieldNotFound {
                        name: columns[i].clone(),
                        container_type: "model",
                        container_name: String::from("unspecified"),
                    }))
                }
            }
        }
    }

    if model_projection.scalar_length() == record_projection.len() {
        Ok(record_projection)
    } else {
        Err(SqlError::DomainError(DomainError::ConversionFailure(
            "ResultSet".to_owned(),
            "RecordProjection".to_owned(),
        )))
    }
}
