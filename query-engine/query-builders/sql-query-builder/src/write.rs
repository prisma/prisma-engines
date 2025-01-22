use crate::limit::wrap_with_limit_subquery_if_needed;
use crate::{model_extensions::*, sql_trace::SqlTraceComment, Context};
use itertools::Itertools;
use quaint::ast::*;
use query_structure::*;
use std::{collections::HashSet, convert::TryInto};

/// `INSERT` a new record to the database. Resulting an `INSERT` ast and an
/// optional `RecordProjection` if available from the arguments or model.
pub fn create_record(
    model: &Model,
    mut args: WriteArgs,
    selected_fields: &ModelProjection,
    ctx: &Context<'_>,
) -> Insert<'static> {
    let fields: Vec<_> = model
        .fields()
        .scalar()
        .filter(|field| args.has_arg_for(field.db_name()))
        .collect();

    let insert = fields
        .into_iter()
        .fold(Insert::single_into(model.as_table(ctx)), |insert, field| {
            let db_name = field.db_name();
            let value = args.take_field_value(db_name).unwrap();
            let value: PrismaValue = value
                .try_into()
                .expect("Create calls can only use PrismaValue write expressions (right now).");

            insert.value(db_name.to_owned(), field.value(value, ctx))
        });

    Insert::from(insert)
        .returning(selected_fields.as_columns(ctx).map(|c| c.set_is_selected(true)))
        .add_traceparent(ctx.traceparent)
}

/// `INSERT` new records into the database based on the given write arguments,
/// where each `WriteArg` in the Vec is one row.
/// Requires `affected_fields` to be non-empty to produce valid SQL.
#[allow(clippy::mutable_key_type)]
pub fn create_records_nonempty(
    model: &Model,
    args: Vec<WriteArgs>,
    skip_duplicates: bool,
    affected_fields: &HashSet<ScalarFieldRef>,
    selected_fields: Option<&ModelProjection>,
    ctx: &Context<'_>,
) -> Insert<'static> {
    // We need to bring all write args into a uniform shape.
    // The easiest way to do this is to take go over all fields of the batch and apply the following:
    // All fields that have a default but are not explicitly provided are inserted with `DEFAULT`.
    let values: Vec<_> = args
        .into_iter()
        .map(|mut arg| {
            let mut row: Vec<Expression> = Vec::with_capacity(affected_fields.len());

            for field in affected_fields.iter() {
                let value = arg.take_field_value(field.db_name());

                match value {
                    Some(write_op) => {
                        let value: PrismaValue = write_op
                            .try_into()
                            .expect("Create calls can only use PrismaValue write expressions (right now).");

                        row.push(field.value(value, ctx).into());
                    }
                    // We can't use `DEFAULT` for SQLite so we provided an explicit `NULL` instead.
                    None if !field.is_required() && field.default_value().is_none() => {
                        row.push(Value::null_int32().raw().into())
                    }
                    None => row.push(default_value()),
                }
            }

            row
        })
        .collect();

    let columns = affected_fields.iter().cloned().collect::<Vec<_>>().as_columns(ctx);
    let insert = Insert::multi_into(model.as_table(ctx), columns);
    let insert = values.into_iter().fold(insert, |stmt, values| stmt.values(values));
    let insert: Insert = insert.into();
    let mut insert = insert.add_traceparent(ctx.traceparent);

    if let Some(selected_fields) = selected_fields {
        insert = insert.returning(projection_into_columns(selected_fields, ctx));
    }

    if skip_duplicates {
        insert = insert.on_conflict(OnConflict::DoNothing)
    }

    insert
}

/// `INSERT` empty records statement.
pub fn create_records_empty(
    model: &Model,
    skip_duplicates: bool,
    selected_fields: Option<&ModelProjection>,
    ctx: &Context<'_>,
) -> Insert<'static> {
    let insert: Insert<'static> = Insert::single_into(model.as_table(ctx)).into();
    let mut insert = insert.add_traceparent(ctx.traceparent);

    if let Some(selected_fields) = selected_fields {
        insert = insert.returning(projection_into_columns(selected_fields, ctx));
    }

    if skip_duplicates {
        insert = insert.on_conflict(OnConflict::DoNothing);
    }

    insert
}

pub fn build_update_and_set_query(
    model: &Model,
    args: WriteArgs,
    selected_fields: Option<&ModelProjection>,
    ctx: &Context<'_>,
) -> Update<'static> {
    let scalar_fields = model.fields().scalar();
    let table = model.as_table(ctx);
    let query = args
        .args
        .into_iter()
        .fold(Update::table(table.clone()), |acc, (field_name, val)| {
            let DatasourceFieldName(name) = field_name;
            let field = scalar_fields
                .clone()
                .find(|f| f.db_name() == name)
                .expect("Expected field to be valid");

            let value: Expression = match val.try_into_scalar().unwrap() {
                ScalarWriteOperation::Field(_) => unimplemented!(),
                ScalarWriteOperation::Set(rhs) => field.value(rhs, ctx).into(),
                ScalarWriteOperation::Add(rhs) if field.is_list() => {
                    let e: Expression = Column::from((table.clone(), name.clone())).into();
                    let vals: Vec<_> = match rhs {
                        PrismaValue::List(vals) => vals.into_iter().map(|val| field.value(val, ctx)).collect(),
                        _ => vec![field.value(rhs, ctx)],
                    };

                    // Postgres only
                    e.compare_raw("||", Value::array(vals)).into()
                }
                ScalarWriteOperation::Add(rhs) => {
                    let e: Expression<'_> = Column::from((table.clone(), name.clone())).into();
                    e + field.value(rhs, ctx).into()
                }

                ScalarWriteOperation::Substract(rhs) => {
                    let e: Expression<'_> = Column::from((table.clone(), name.clone())).into();
                    e - field.value(rhs, ctx).into()
                }

                ScalarWriteOperation::Multiply(rhs) => {
                    let e: Expression<'_> = Column::from((table.clone(), name.clone())).into();
                    e * field.value(rhs, ctx).into()
                }

                ScalarWriteOperation::Divide(rhs) => {
                    let e: Expression<'_> = Column::from((table.clone(), name.clone())).into();
                    e / field.value(rhs, ctx).into()
                }

                ScalarWriteOperation::Unset(_) => unreachable!("Unset is not supported on SQL connectors"),
            };

            acc.set(name, value)
        });

    let query = query.add_traceparent(ctx.traceparent);

    let query = if let Some(selected_fields) = selected_fields {
        query.returning(selected_fields.as_columns(ctx).map(|c| c.set_is_selected(true)))
    } else {
        query
    };

    query
}

pub fn chunk_update_with_ids(
    update: Update<'static>,
    model: &Model,
    ids: &[SelectionResult],
    filter_condition: ConditionTree<'static>,
    ctx: &Context<'_>,
) -> Vec<Query<'static>> {
    let columns: Vec<_> = ModelProjection::from(model.primary_identifier())
        .as_columns(ctx)
        .collect();

    super::chunked_conditions(&columns, ids, ctx, |conditions| {
        update.clone().so_that(conditions.and(filter_condition.clone()))
    })
}

/// Converts a list of selected fields into an iterator of table columns.
fn projection_into_columns(
    selected_fields: &ModelProjection,
    ctx: &Context<'_>,
) -> impl Iterator<Item = Column<'static>> {
    selected_fields.as_columns(ctx).map(|c| c.set_is_selected(true))
}

pub fn delete_returning(
    model: &Model,
    filter: ConditionTree<'static>,
    selected_fields: &ModelProjection,
    ctx: &Context<'_>,
) -> Query<'static> {
    Delete::from_table(model.as_table(ctx))
        .so_that(filter)
        .returning(projection_into_columns(selected_fields, ctx))
        .add_traceparent(ctx.traceparent)
        .into()
}

pub fn delete_many_from_filter(
    model: &Model,
    filter_condition: ConditionTree<'static>,
    limit: Option<usize>,
    ctx: &Context<'_>,
) -> Query<'static> {
    let filter_condition = wrap_with_limit_subquery_if_needed(model, filter_condition, limit, ctx);

    Delete::from_table(model.as_table(ctx))
        .so_that(filter_condition)
        .add_traceparent(ctx.traceparent)
        .into()
}

pub fn delete_many_from_ids_and_filter(
    model: &Model,
    ids: &[SelectionResult],
    filter_condition: ConditionTree<'static>,
    limit: Option<usize>,
    ctx: &Context<'_>,
) -> Vec<Query<'static>> {
    let columns: Vec<_> = ModelProjection::from(model.primary_identifier())
        .as_columns(ctx)
        .collect();

    super::chunked_conditions(&columns, ids, ctx, |conditions| {
        delete_many_from_filter(model, conditions.and(filter_condition.clone()), limit, ctx)
    })
}

pub fn create_relation_table_records(
    field: &RelationFieldRef,
    parent_id: &SelectionResult,
    child_ids: &[SelectionResult],
    ctx: &Context<'_>,
) -> Query<'static> {
    let relation = field.relation();

    let parent_columns: Vec<_> = field.related_field().m2m_columns(ctx);
    let child_columns: Vec<_> = field.m2m_columns(ctx);

    let columns: Vec<_> = parent_columns.into_iter().chain(child_columns).collect();
    let insert = Insert::multi_into(relation.as_table(ctx), columns);

    let insert: MultiRowInsert = child_ids.iter().fold(insert, |insert, child_id| {
        let mut values: Vec<_> = parent_id.db_values(ctx);

        values.extend(child_id.db_values(ctx));
        insert.values(values)
    });

    // NOTE: There is no comment support for MultiRowInsert
    insert.build().on_conflict(OnConflict::DoNothing).into()
}

pub fn delete_relation_table_records(
    parent_field: &RelationFieldRef,
    parent_id: &SelectionResult,
    child_ids: &[SelectionResult],
    ctx: &Context<'_>,
) -> Delete<'static> {
    let relation = parent_field.relation();

    let mut parent_columns: Vec<_> = parent_field.related_field().m2m_columns(ctx);
    let child_columns: Vec<_> = parent_field.m2m_columns(ctx);

    let parent_id_values = parent_id.db_values(ctx);
    let parent_id_criteria = if parent_columns.len() > 1 {
        Row::from(parent_columns).equals(parent_id_values)
    } else {
        parent_columns.pop().unwrap().equals(parent_id_values)
    };

    let child_id_criteria = super::in_conditions(&child_columns, child_ids, ctx);

    Delete::from_table(relation.as_table(ctx))
        .so_that(parent_id_criteria.and(child_id_criteria))
        .add_traceparent(ctx.traceparent)
}

/// Generates a list of insert statements to execute. If `selected_fields` is set, insert statements
/// will return the specified columns of inserted rows.
pub fn generate_insert_statements(
    model: &Model,
    args: Vec<WriteArgs>,
    skip_duplicates: bool,
    selected_fields: Option<&ModelProjection>,
    ctx: &Context<'_>,
) -> Vec<Insert<'static>> {
    let affected_fields = collect_affected_fields(&args, model);

    if affected_fields.is_empty() {
        args.into_iter()
            .map(|_| create_records_empty(model, skip_duplicates, selected_fields, ctx))
            .collect()
    } else {
        let partitioned_batches = partition_into_batches(args, ctx);

        partitioned_batches
            .into_iter()
            .map(|batch| create_records_nonempty(model, batch, skip_duplicates, &affected_fields, selected_fields, ctx))
            .collect()
    }
}

/// Returns a set of fields that are used in the arguments for the create operation.
fn collect_affected_fields(args: &[WriteArgs], model: &Model) -> HashSet<ScalarFieldRef> {
    let mut fields = HashSet::new();
    args.iter().for_each(|arg| fields.extend(arg.keys()));

    fields
        .into_iter()
        .map(|dsfn| model.fields().scalar().find(|sf| sf.db_name() == &**dsfn).unwrap())
        .collect()
}

/// Partitions data into batches, respecting `max_bind_values` and `max_insert_rows` settings from
/// the `Context`.
fn partition_into_batches(args: Vec<WriteArgs>, ctx: &Context<'_>) -> Vec<Vec<WriteArgs>> {
    let batches = if let Some(max_params) = ctx.max_bind_values() {
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

    if let Some(max_rows) = ctx.max_insert_rows() {
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
