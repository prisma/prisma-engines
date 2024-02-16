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
            VirtualSelection::RelationCount(rc) => {
                let join_alias = format!("aggr_selection_{index}");
                let aggregator_alias = selection.db_alias();
                let join = compute_aggr_join(
                    rc.field(),
                    AggregationType::Count,
                    rc.filter().cloned(),
                    aggregator_alias.as_str(),
                    join_alias.as_str(),
                    None,
                    ctx,
                );

                columns.push(Column::from((join.alias.clone(), aggregator_alias)).into());
                joins.push(join);
            }
        }
    }

    RelAggregationJoins { joins, columns }
}
