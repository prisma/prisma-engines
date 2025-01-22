use quaint::ast::Query;
use query_structure::{Filter, Model, ModelProjection, RecordFilter, SelectionResult, WriteArgs};

use crate::{limit, write, AsColumns, Context, FilterBuilder};

// Generates a query like this:
//  UPDATE "public"."User" SET "name" = $1 WHERE "public"."User"."age" > $1
pub fn update_many_from_filter(
    model: &Model,
    record_filter: RecordFilter,
    args: WriteArgs,
    selected_fields: Option<&ModelProjection>,
    limit: Option<usize>,
    ctx: &Context<'_>,
) -> Query<'static> {
    let update = write::build_update_and_set_query(model, args, None, ctx);
    let filter_condition = limit::wrap_with_limit_subquery_if_needed(
        model,
        FilterBuilder::without_top_level_joins().visit_filter(record_filter.filter, ctx),
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
