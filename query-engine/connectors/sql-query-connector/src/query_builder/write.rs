use crate::{model_extensions::*, sql_trace::SqlTraceComment, Context};
use connector_interface::{DatasourceFieldName, ScalarWriteOperation, WriteArgs};
use prisma_models::*;
use quaint::ast::*;
use std::{collections::HashSet, convert::TryInto};
use tracing::Span;

/// `INSERT` a new record to the database. Resulting an `INSERT` ast and an
/// optional `RecordProjection` if available from the arguments or model.
pub(crate) fn create_record(model: &ModelRef, mut args: WriteArgs, ctx: &Context<'_>) -> Insert<'static> {
    let fields: Vec<_> = model
        .fields()
        .scalar()
        .into_iter()
        .filter(|field| args.has_arg_for(&field.db_name()))
        .collect();

    let insert = fields
        .into_iter()
        .fold(Insert::single_into(model.as_table(ctx)), |insert, field| {
            let db_name = field.db_name();
            let value = args.take_field_value(db_name).unwrap();
            let value: PrismaValue = value
                .try_into()
                .expect("Create calls can only use PrismaValue write expressions (right now).");

            insert.value(db_name.to_owned(), field.value(value))
        });

    Insert::from(insert)
        .returning(ModelProjection::from(model.primary_identifier()).as_columns(ctx))
        .append_trace(&Span::current())
        .add_trace_id(ctx.trace_id)
}

/// `INSERT` new records into the database based on the given write arguments,
/// where each `WriteArg` in the Vec is one row.
/// Requires `affected_fields` to be non-empty to produce valid SQL.
#[allow(clippy::mutable_key_type)]
pub(crate) fn create_records_nonempty(
    model: &ModelRef,
    args: Vec<WriteArgs>,
    skip_duplicates: bool,
    affected_fields: &HashSet<ScalarFieldRef>,
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

                        row.push(field.value(value).into());
                    }

                    None => row.push(default_value()),
                }
            }

            row
        })
        .collect();

    let columns = affected_fields
        .iter()
        .map(Clone::clone)
        .collect::<Vec<_>>()
        .as_columns(ctx);
    let insert = Insert::multi_into(model.as_table(ctx), columns);
    let insert = values.into_iter().fold(insert, |stmt, values| stmt.values(values));
    let insert: Insert = insert.into();
    let insert = insert.append_trace(&Span::current()).add_trace_id(ctx.trace_id);

    if skip_duplicates {
        insert.on_conflict(OnConflict::DoNothing)
    } else {
        insert
    }
}

/// `INSERT` empty records statement.
pub(crate) fn create_records_empty(model: &ModelRef, skip_duplicates: bool, ctx: &Context<'_>) -> Insert<'static> {
    let insert: Insert<'static> = Insert::single_into(model.as_table(ctx)).into();
    let insert = insert.append_trace(&Span::current()).add_trace_id(ctx.trace_id);

    if skip_duplicates {
        insert.on_conflict(OnConflict::DoNothing)
    } else {
        insert
    }
}

pub(crate) fn build_update_and_set_query(model: &ModelRef, args: WriteArgs, ctx: &Context<'_>) -> Update<'static> {
    let scalar_fields = model.fields().scalar();
    let table = model.as_table(ctx);
    let query = args
        .args
        .into_iter()
        .fold(Update::table(table.clone()), |acc, (field_name, val)| {
            let DatasourceFieldName(name) = field_name;
            let field = scalar_fields
                .iter()
                .find(|f| f.db_name() == name)
                .expect("Expected field to be valid");

            let value: Expression = match val.try_into_scalar().unwrap() {
                ScalarWriteOperation::Field(_) => unimplemented!(),
                ScalarWriteOperation::Set(rhs) => field.value(rhs).into(),
                ScalarWriteOperation::Add(rhs) if field.is_list() => {
                    let e: Expression = Column::from((table.clone(), name.clone())).into();
                    let vals: Vec<_> = match rhs {
                        PrismaValue::List(vals) => vals.into_iter().map(|val| field.value(val)).collect(),
                        _ => vec![field.value(rhs)],
                    };

                    // Postgres only
                    e.compare_raw("||", Value::array(vals)).into()
                }
                ScalarWriteOperation::Add(rhs) => {
                    let e: Expression<'_> = Column::from((table.clone(), name.clone())).into();
                    e + field.value(rhs).into()
                }

                ScalarWriteOperation::Substract(rhs) => {
                    let e: Expression<'_> = Column::from((table.clone(), name.clone())).into();
                    e - field.value(rhs).into()
                }

                ScalarWriteOperation::Multiply(rhs) => {
                    let e: Expression<'_> = Column::from((table.clone(), name.clone())).into();
                    e * field.value(rhs).into()
                }

                ScalarWriteOperation::Divide(rhs) => {
                    let e: Expression<'_> = Column::from((table.clone(), name.clone())).into();
                    e / field.value(rhs).into()
                }

                ScalarWriteOperation::Unset(_) => unreachable!("Unset is not supported on SQL connectors"),
            };

            acc.set(name, value)
        });

    query.append_trace(&Span::current()).add_trace_id(ctx.trace_id)
}

pub(crate) fn chunk_update_with_ids(
    update: Update<'static>,
    model: &ModelRef,
    ids: &[&SelectionResult],
    filter_condition: ConditionTree<'static>,
    ctx: &Context<'_>,
) -> crate::Result<Vec<Query<'static>>> {
    let columns: Vec<_> = ModelProjection::from(model.primary_identifier())
        .as_columns(ctx)
        .collect();

    let query = super::chunked_conditions(&columns, ids, |conditions| {
        update.clone().so_that(conditions.and(filter_condition.clone()))
    });

    Ok(query)
}

pub(crate) fn delete_many(
    model: &ModelRef,
    ids: &[&SelectionResult],
    filter_condition: ConditionTree<'static>,
    ctx: &Context<'_>,
) -> Vec<Query<'static>> {
    let columns: Vec<_> = ModelProjection::from(model.primary_identifier())
        .as_columns(ctx)
        .collect();

    super::chunked_conditions(&columns, ids, |conditions| {
        Delete::from_table(model.as_table(ctx))
            .so_that(conditions.and(filter_condition.clone()))
            .append_trace(&Span::current())
            .add_trace_id(ctx.trace_id)
    })
}

pub(crate) fn create_relation_table_records(
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
        let mut values: Vec<_> = parent_id.db_values();

        values.extend(child_id.db_values());
        insert.values(values)
    });

    // NOTE: There is no comment support for MultiRowInsert
    insert.build().on_conflict(OnConflict::DoNothing).into()
}

pub(crate) fn delete_relation_table_records(
    parent_field: &RelationFieldRef,
    parent_id: &SelectionResult,
    child_ids: &[SelectionResult],
    ctx: &Context<'_>,
) -> Delete<'static> {
    let relation = parent_field.relation();

    let mut parent_columns: Vec<_> = parent_field.related_field().m2m_columns(ctx);
    let child_columns: Vec<_> = parent_field.m2m_columns(ctx);

    let parent_id_values = parent_id.db_values();
    let parent_id_criteria = if parent_columns.len() > 1 {
        Row::from(parent_columns).equals(parent_id_values)
    } else {
        parent_columns.pop().unwrap().equals(parent_id_values)
    };

    let child_id_criteria = super::conditions(&child_columns, child_ids);

    Delete::from_table(relation.as_table(ctx))
        .so_that(parent_id_criteria.and(child_id_criteria))
        .append_trace(&Span::current())
        .add_trace_id(ctx.trace_id)
}
