use crate::{
    join_utils::{compute_aggr_join, AggregationType, AliasedJoin},
    Context,
};
use quaint::prelude::*;
use query_structure::VirtualSelection;

#[derive(Debug)]
pub(crate) struct RelAggregationJoins {
    // Joins necessary to perform the relation aggregations
    pub(crate) joins: Vec<AliasedJoin>,
    // Aggregator columns
    pub(crate) columns: Vec<Expression<'static>>,
}

pub(crate) fn build<'a>(
    virtual_selections: impl IntoIterator<Item = &'a VirtualSelection>,
    ctx: &Context<'_>,
) -> RelAggregationJoins {
    let mut joins = vec![];
    let mut columns: Vec<Expression<'static>> = vec![];

    for (index, selection) in virtual_selections.into_iter().enumerate() {
        match selection {
            VirtualSelection::RelationCount(rf, filter) => {
                let join_alias = format!("aggr_selection_{index}");
                let aggregator_alias = selection.db_alias();
                let join = compute_aggr_join(
                    rf,
                    AggregationType::Count,
                    filter.clone(),
                    aggregator_alias.as_str(),
                    join_alias.as_str(),
                    None,
                    ctx,
                );

                let exprs: Vec<Expression> = vec![
                    Column::from((join.alias.clone(), aggregator_alias.clone())).into(),
                    Value::int64(0).raw().into(),
                ];

                // We coalesce the COUNT to 0 so that if there's no relation,
                // `COALESCE(NULL, 0)` will return `0`, thus avoiding
                // https://github.com/prisma/prisma/issues/23778 in Turso.
                // We also need to add the alias to the COALESCE'd column explicitly,
                // to reference it later.
                columns.push(Expression::from(coalesce(exprs)).alias(aggregator_alias));
                joins.push(join);
            }
        }
    }

    RelAggregationJoins { joins, columns }
}
