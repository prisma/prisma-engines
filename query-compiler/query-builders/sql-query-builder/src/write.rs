use crate::limit::wrap_with_limit_subquery_if_needed;
use crate::{Context, model_extensions::*, sql_trace::SqlTraceComment};
use crate::{FilterBuilder, update};
use itertools::Itertools;
use quaint::ast::*;
use query_structure::*;
use std::collections::HashMap;
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
    let mut fields = affected_fields.iter().cloned().collect_vec();
    fields.sort_by_key(|f| f.id);

    // We need to bring all write args into a uniform shape.
    // The easiest way to do this is to take go over all fields of the batch and apply the following:
    // All fields that have a default but are not explicitly provided are inserted with `DEFAULT`.
    let values: Vec<_> = args
        .into_iter()
        .map(|mut arg| {
            let mut row: Vec<Expression> = Vec::with_capacity(fields.len());

            for field in fields.iter() {
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

    let columns = fields.as_columns(ctx);
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

/// `INSERT` with `ON CONFLICT DO UPDATE` statement.
pub fn native_upsert(
    model: &Model,
    filter: Filter,
    create_args: WriteArgs,
    update_args: WriteArgs,
    selected_fields: &ModelProjection,
    unique_constraints: &[ScalarFieldRef],
    ctx: &Context<'_>,
) -> Insert<'static> {
    let where_condition = FilterBuilder::without_top_level_joins().visit_filter(filter, ctx);
    let update = build_update_and_set_query(model, update_args, None, ctx).so_that(where_condition);
    let insert = create_record(model, create_args, selected_fields, ctx);

    let constraints: Vec<_> = unique_constraints.as_columns(ctx).collect();
    insert.on_conflict(OnConflict::Update(update, constraints))
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
                    let arr = match rhs {
                        PrismaValue::List(vals) => Value::array(vals.into_iter().map(|val| field.value(val, ctx))),
                        PrismaValue::Placeholder(ref ph) if matches!(ph.r#type, PrismaValueType::List(_)) => {
                            field.value(rhs, ctx)
                        }
                        _ => Value::array(vec![field.value(rhs, ctx)]),
                    };

                    // Postgres only
                    e.compare_raw("||", arr).into()
                }
                ScalarWriteOperation::Add(rhs) => {
                    let e: Expression<'_> = Column::from((table.clone(), name.clone())).into();
                    e + field.value(rhs, ctx).into()
                }

                ScalarWriteOperation::Subtract(rhs) => {
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

    if let Some(selected_fields) = selected_fields {
        query.returning(selected_fields.as_columns(ctx).map(|c| c.set_is_selected(true)))
    } else {
        query
    }
}

pub fn chunk_update_with_ids(
    update: Update<'static>,
    model: &Model,
    ids: &[SelectionResult],
    filter_condition: ConditionTree<'static>,
    ctx: &Context<'_>,
) -> Vec<Query<'static>> {
    let columns: Vec<_> = ModelProjection::from(model.shard_aware_primary_identifier())
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

pub fn generate_update_statements(
    model: &Model,
    record_filter: RecordFilter,
    args: WriteArgs,
    selected_fields: Option<&ModelProjection>,
    limit: Option<usize>,
    ctx: &Context<'_>,
) -> Vec<Query<'static>> {
    let RecordFilter { filter, selectors } = record_filter;
    match selectors {
        Some(ids) => {
            let slice = &ids[..limit.unwrap_or(ids.len()).min(ids.len())];
            update::update_many_from_ids_and_filter(model, filter, slice, args, selected_fields, ctx)
        }
        None => {
            let query = update::update_many_from_filter(model, filter, args, selected_fields, limit, ctx);
            vec![query]
        }
    }
}

/// Generates deletes for multiple records, defined in the `RecordFilter`.
pub fn generate_delete_statements(
    model: &Model,
    record_filter: RecordFilter,
    limit: Option<usize>,
    ctx: &Context<'_>,
) -> Vec<Query<'static>> {
    let filter_condition = FilterBuilder::without_top_level_joins().visit_filter(record_filter.filter.clone(), ctx);

    // If we have selectors, then we must chunk the mutation into multiple if necessary and add the ids to the filter.
    if let Some(selectors) = record_filter.selectors.as_deref() {
        let slice = &selectors[..limit.unwrap_or(selectors.len()).min(selectors.len())];
        delete_many_from_ids_and_filter(model, slice, filter_condition, limit, ctx)
    } else {
        vec![delete_many_from_filter(model, filter_condition, limit, ctx)]
    }
}

pub fn delete_returning(
    model: &Model,
    filter: Filter,
    selected_fields: &ModelProjection,
    ctx: &Context<'_>,
) -> Query<'static> {
    let filter = FilterBuilder::without_top_level_joins().visit_filter(filter, ctx);

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
    let columns: Vec<_> = ModelProjection::from(model.shard_aware_primary_identifier())
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

    let parent_column = field.related_field().m2m_column(ctx);
    let child_column = field.m2m_column(ctx);

    let insert = Insert::multi_into(relation.as_table(ctx), vec![parent_column, child_column]);

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

    let parent_column = parent_field.related_field().m2m_column(ctx);
    let child_column = parent_field.m2m_column(ctx);

    let parent_id_values = parent_id.db_values(ctx);
    let parent_id_criteria = parent_column.equals(parent_id_values);

    let child_ids_row = child_ids.iter().flat_map(|id| id.db_values(ctx)).collect::<Row>();

    let child_id_criteria = if !child_ids.is_empty()
        && child_ids[0]
            .pairs
            .iter()
            .any(|(_, pv)| matches!(pv, PrismaValue::Placeholder { .. }))
    {
        child_column.in_selection(child_ids_row.to_parameterized_row())
    } else {
        child_column.in_selection(child_ids_row)
    };

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
///
/// We need to split inserts if they are above a parameter threshold, as well as split based on number of rows:
/// horizontal partitioning by row number, vertical by number of args.
fn partition_into_batches(args: Vec<WriteArgs>, ctx: &Context<'_>) -> Vec<Vec<WriteArgs>> {
    let batches = if let Some(max_params) = ctx.max_bind_values() {
        #[derive(Default)]
        struct Batch {
            items: Vec<WriteArgs>,
            param_count: usize,
        }

        impl Batch {
            fn add(&mut self, item: WriteArgs) {
                let len = item.len();
                self.items.push(item);
                self.param_count += len;
            }
        }

        impl From<WriteArgs> for Batch {
            fn from(args: WriteArgs) -> Self {
                let mut batch = Self::default();
                batch.add(args);
                batch
            }
        }

        args.into_iter()
            .fold(Vec::<Batch>::new(), |mut acc, item| {
                if let Some(last_batch) = acc.last_mut() {
                    if last_batch.param_count + item.len() > max_params {
                        acc.push(item.into());
                    } else {
                        last_batch.add(item);
                    }
                } else {
                    acc.push(item.into());
                }
                acc
            })
            .into_iter()
            .map(|batch| batch.items)
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

/// Returns a list of fields that can be used to group CreateMany entries optimally.
///
/// This is needed for connectors that don't support the `DEFAULT` expression when inserting records in bulk.
/// `DEFAULT` is needed for fields that have a default value that the QueryEngine cannot generate at runtime (@autoincrement(), @dbgenerated()).
///
/// Two CreateMany entries cannot be grouped together when they contain different fields that require the use of a `DEFAULT` expression.
/// - When they have the same set of fields that require `DEFAULT`, those fields can be ommited entirely from the `INSERT` expression, in which case `DEFAULT` is implied.
/// - When they don't, since all `VALUES` entries of the `INSERT` expression must be the same, we have to split the CreateMany entries into separate `INSERT` expressions.
///
/// Consequently, if a field has a default value and is _not_ present in the [`WriteArgs`], this constitutes a discriminant that can be used to group CreateMany entries.
///
/// As such, the fields that we compute for a given CreateMany entry is the set of fields that are _not_ present in the [`WriteArgs`] and that have a default value.
/// Note: This works because the [`crate::QueryDocumentParser`] injects into the CreateMany entries, the default values that _can_ be generated at runtime.
/// Note: We can ignore optional fields without default values because they can be inserted as `NULL`. It is a value that the QueryEngine _can_ generate at runtime.
pub fn split_write_args_by_shape(model: &Model, args: Vec<WriteArgs>) -> impl Iterator<Item = Vec<WriteArgs>> {
    let mut args_by_shape: HashMap<_, Vec<_>> = HashMap::new();
    for write_args in args {
        let shape = write_args_to_shape(&write_args, model);
        args_by_shape.entry(shape).or_default().push(write_args);
    }
    args_by_shape.into_values()
}

fn write_args_to_shape(write_args: &WriteArgs, model: &Model) -> Vec<DatasourceFieldName> {
    let mut shape = Vec::new();

    for field in model.fields().scalar() {
        if !write_args.args.contains_key(field.db_name()) && field.default_value().is_some() {
            shape.push(DatasourceFieldName(field.db_name().to_string()));
        }
    }

    // This ensures that shapes are not dependent on order of fields.
    shape.sort_unstable();

    shape
}

pub fn defaults_for_mysql_write_args<'a>(
    id_field: &'a FieldSelection,
    args: &'a WriteArgs,
) -> impl Iterator<Item = (ScalarField, Expression<'static>)> + use<'a> {
    // Go through all the values and generate a select statement with the correct MySQL function
    id_field.selections().filter_map(|field| {
        let (sf, func) = match field {
            SelectedField::Scalar(sf) if !args.has_arg_for(sf.db_name()) => {
                (sf, sf.default_value()?.to_dbgenerated_func()?)
            }
            _ => return None,
        };
        let alias = field.db_name().into_owned();
        let func = func.to_lowercase().replace(' ', "");

        match func.as_str() {
            "(uuid())" => Some((sf.clone(), native_uuid().alias(alias))),
            "(uuid_to_bin(uuid()))" | "(uuid_to_bin(uuid(),0))" => Some((sf.clone(), uuid_to_bin().alias(alias))),
            "(uuid_to_bin(uuid(),1))" => Some((sf.clone(), uuid_to_bin_swapped().alias(alias))),
            _ => None,
        }
    })
}
