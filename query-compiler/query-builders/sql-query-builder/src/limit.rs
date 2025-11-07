use crate::{Context, model_extensions::*};
use quaint::ast::*;
use query_structure::*;

pub fn wrap_with_limit_subquery_if_needed<'a>(
    model: &Model,
    filter_condition: ConditionTree<'a>,
    limit: Option<usize>,
    ctx: &Context,
) -> ConditionTree<'a> {
    if let Some(limit) = limit {
        let columns = model
            .shard_aware_primary_identifier()
            .as_scalar_fields()
            .expect("primary identifier must contain scalar fields")
            .into_iter()
            .map(|f| f.as_column(ctx))
            .collect::<Vec<_>>();

        ConditionTree::from(
            Row::from(columns.clone()).in_selection(
                Select::from_table(model.as_table(ctx))
                    .columns(columns)
                    .so_that(filter_condition)
                    .limit(limit),
            ),
        )
    } else {
        filter_condition
    }
}
