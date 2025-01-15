use super::read::get_single_record;

use crate::row::ToSqlRow;
use crate::{QueryExt, Queryable};

use itertools::Itertools;
use quaint::ast::*;
use query_structure::*;
use sql_query_builder::{column_metadata, limit, write, AsColumns, ColumnMetadata, Context, FilterBuilder};

/// Performs an update with an explicit selection set.
/// This function is called for connectors that supports the `UpdateReturning` capability.
pub(crate) async fn update_one_with_selection(
    conn: &dyn Queryable,
    model: &Model,
    record_filter: RecordFilter,
    args: WriteArgs,
    selected_fields: FieldSelection,
    ctx: &Context<'_>,
) -> crate::Result<Option<SingleRecord>> {
    // If there's nothing to update, just read the record.
    // TODO(perf): Technically, if the selectors are fulfilling the field selection, there's no need to perform an additional read.
    if args.args.is_empty() {
        let filter = build_update_one_filter(record_filter);
        return get_single_record(conn, model, &filter, &selected_fields, RelationLoadStrategy::Query, ctx).await;
    }

    let selected_fields = ModelProjection::from(selected_fields);

    let cond = FilterBuilder::without_top_level_joins().visit_filter(build_update_one_filter(record_filter), ctx);

    let update = write::build_update_and_set_query(model, args, Some(&selected_fields), ctx).so_that(cond);

    let field_names: Vec<_> = selected_fields.db_names().collect();
    let idents = selected_fields.type_identifiers_with_arities();
    let meta = column_metadata::create(&field_names, &idents);

    let result_row = conn.query(update.into()).await?.into_iter().next();
    let record = result_row
        .map(|row| process_result_row(row, &meta, &selected_fields))
        .transpose()?
        .map(|selection| SingleRecord {
            record: Record::from(selection),
            field_names: selected_fields.db_names().collect(),
        });

    Ok(record)
}

/// Performs an update without an explicit selection set.
/// This function is called for connectors lacking the `UpdateReturning` capability.
/// As we don't have a selection set to work with, this function always returns a record with the primary identifier of the model (provided that a record was found).
/// However, since we can't get the updated values back from the update operation, we need to read the primary identifier _before_ the update and then update the ids in-memory if they were updated.
pub(crate) async fn update_one_without_selection(
    conn: &dyn Queryable,
    model: &Model,
    record_filter: RecordFilter,
    args: WriteArgs,
    ctx: &Context<'_>,
) -> crate::Result<Option<SingleRecord>> {
    // If there's nothing to update, just return the ids.
    // If the parent operation did not pass any ids, then perform a read so that the following operations can be resolved.
    if args.args.is_empty() {
        let ids: Vec<SelectionResult> = conn.filter_selectors(model, record_filter.clone(), ctx).await?;

        let record = ids.into_iter().next().map(|id| SingleRecord {
            record: Record::from(id),
            field_names: model.primary_identifier().db_names().collect_vec(),
        });

        return Ok(record);
    }

    // Pick the primary identifiers args from the WriteArgs if there are any.
    let id_args = pick_args(&model.primary_identifier().into(), &args);
    // Perform the update and return the ids on which we've applied the update.
    // Note: We are _not_ getting back the ids from the update. Either we got some ids passed from the parent operation or we perform a read _before_ doing the update.
    let (updates, ids) = update_many_from_ids_and_filter(conn, model, record_filter, args, None, None, ctx).await?;
    for update in updates {
        conn.execute(update).await?;
    }

    // Since we could not get the ids back from the update, we need to apply in-memory transformation to the ids in case they were part of the update.
    // This is critical to ensure the following operations can operate on the updated ids.
    let merged_ids = merge_write_args(ids, id_args);

    let record = merged_ids.into_iter().next().map(|id| SingleRecord {
        record: Record::from(id),
        field_names: model.primary_identifier().db_names().collect(),
    });

    Ok(record)
}

// Generates a query like this:
//  UPDATE "public"."User" SET "name" = $1 WHERE "public"."User"."age" > $1
pub(super) async fn update_many_from_filter(
    model: &Model,
    record_filter: RecordFilter,
    args: WriteArgs,
    selected_fields: Option<&ModelProjection>,
    limit: Option<usize>,
    ctx: &Context<'_>,
) -> crate::Result<Query<'static>> {
    let update = write::build_update_and_set_query(model, args, None, ctx);
    let filter_condition = limit::wrap_with_limit_subquery_if_needed(
        model,
        FilterBuilder::without_top_level_joins().visit_filter(record_filter.filter, ctx),
        limit,
        ctx,
    );

    let update = update.so_that(filter_condition);
    if let Some(selected_fields) = selected_fields {
        Ok(update
            .returning(selected_fields.as_columns(ctx).map(|c| c.set_is_selected(true)))
            .into())
    } else {
        Ok(update.into())
    }
}

// Generates a query like this:
//  UPDATE "public"."User" SET "name" = $1 WHERE "public"."User"."id" IN ($2,$3,$4,$5,$6,$7,$8,$9,$10,$11) AND "public"."User"."age" > $1
pub(super) async fn update_many_from_ids_and_filter(
    conn: &dyn Queryable,
    model: &Model,
    record_filter: RecordFilter,
    args: WriteArgs,
    selected_fields: Option<&ModelProjection>,
    limit: Option<usize>,
    ctx: &Context<'_>,
) -> crate::Result<(Vec<Query<'static>>, Vec<SelectionResult>)> {
    let filter_condition = FilterBuilder::without_top_level_joins().visit_filter(record_filter.filter.clone(), ctx);
    let ids: Vec<SelectionResult> = conn.filter_selectors(model, record_filter, ctx).await?;

    if ids.is_empty() {
        return Ok((vec![], Vec::new()));
    }

    let updates = {
        let update = write::build_update_and_set_query(model, args, selected_fields, ctx);
        let ids: Vec<&SelectionResult> = ids.iter().take(limit.unwrap_or(usize::MAX)).collect();

        write::chunk_update_with_ids(update, model, &ids, filter_condition, ctx)
    };

    Ok((updates, ids))
}

fn process_result_row(
    row: quaint::prelude::ResultRow,
    meta: &[ColumnMetadata<'_>],
    selected_fields: &ModelProjection,
) -> crate::Result<SelectionResult> {
    let sql_row = row.to_sql_row(meta)?;
    let prisma_row = selected_fields.scalar_fields().zip(sql_row.values).collect_vec();

    Ok(SelectionResult::new(prisma_row))
}

/// Given a record filter, builds a ConditionTree composed of:
/// 1. The `RecordFilter.filter`
/// 2. The `RecordFilter.selectors`, if any are present, transformed to an `In()` filter
///
/// Both filters are 'AND'ed.
///
/// Note: This function should only be called for update_one filters. It is not chunking the filters into multiple queries.
/// Note: Using this function to render an update_many filter could exceed the maximum query parameters available for a connector.
fn build_update_one_filter(record_filter: RecordFilter) -> Filter {
    match record_filter.selectors {
        Some(selectors) => Filter::and(vec![selectors.filter(), record_filter.filter]),
        None => record_filter.filter,
    }
}
