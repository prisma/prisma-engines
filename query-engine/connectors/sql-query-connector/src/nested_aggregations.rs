use crate::{
    join_utils::{compute_aggr_join, AggregationType, AliasedJoin},
    Context,
};
use connector_interface::RelAggregationSelection;
use quaint::prelude::*;

#[derive(Debug)]
pub(crate) struct RelAggregationJoins {
    // Joins necessary to perform the relation aggregations
    pub(crate) joins: Vec<AliasedJoin>,
    // Aggregator columns
    pub(crate) columns: Vec<Expression<'static>>,
}

pub(crate) fn build(aggr_selections: &[RelAggregationSelection], ctx: &Context<'_>) -> RelAggregationJoins {
    let mut joins = vec![];
    let mut columns: Vec<Expression<'static>> = vec![];

    for (index, selection) in aggr_selections.iter().enumerate() {
        match selection {
            RelAggregationSelection::Count(rf, filter) => {
                let join_alias = format!("aggr_selection_{}", index);
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

                columns.push(Column::from((join.alias.clone(), aggregator_alias)).into());
                joins.push(join);
            }
        }
    }

    RelAggregationJoins { joins, columns }
}
