use quaint::ast::{Query, Update};
use query_structure::{Filter, IntoFilter, Model, ModelProjection, RecordFilter, SelectionResult, WriteArgs};

use crate::{AsColumns, Context, FilterBuilder, limit, write};

// Generates a query like this:
//  UPDATE "public"."User" SET "name" = $1 WHERE "public"."User"."age" > $1
pub fn update_many_from_filter(
    model: &Model,
    filter: Filter,
    args: WriteArgs,
    selected_fields: Option<&ModelProjection>,
    limit: Option<usize>,
    ctx: &Context<'_>,
) -> Query<'static> {
    let update = write::build_update_and_set_query(model, args, None, ctx);
    let filter_condition = limit::wrap_with_limit_subquery_if_needed(
        model,
        FilterBuilder::without_top_level_joins().visit_filter(filter, ctx),
        limit,
        ctx,
    );

    let update = update.so_that(filter_condition);
    if let Some(selected_fields) = selected_fields {
        update
            .returning(selected_fields.as_columns(ctx).map(|c| c.set_is_selected(true)))
            .into()
    } else {
        update.into()
    }
}

// Generates a query like this:
//  UPDATE "public"."User" SET "name" = $1 WHERE "public"."User"."id" IN ($2,$3,$4,$5,$6,$7,$8,$9,$10,$11) AND "public"."User"."age" > $1
pub fn update_many_from_ids_and_filter(
    model: &Model,
    filter: Filter,
    selections: &[SelectionResult],
    args: WriteArgs,
    selected_fields: Option<&ModelProjection>,
    ctx: &Context<'_>,
) -> Vec<Query<'static>> {
    let filter_condition = FilterBuilder::without_top_level_joins().visit_filter(filter, ctx);

    if selections.is_empty() {
        return vec![];
    }

    let update = write::build_update_and_set_query(model, args, selected_fields, ctx);
    write::chunk_update_with_ids(update, model, selections, filter_condition, ctx)
}

/// Creates an update with an explicit selection set.
pub fn update_one_with_selection(
    model: &Model,
    record_filter: RecordFilter,
    args: WriteArgs,
    selected_fields: &ModelProjection,
    ctx: &Context<'_>,
) -> Update<'static> {
    let cond = FilterBuilder::without_top_level_joins().visit_filter(build_update_one_filter(record_filter), ctx);
    write::build_update_and_set_query(model, args, Some(selected_fields), ctx).so_that(cond)
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
