use crate::{
    join_utils::{AggregationType, AliasedJoin},
    query_builder::QueryBuilderContext,
};
use connector_interface::RelAggregationSelection;
use quaint::prelude::*;

#[derive(Debug)]
pub struct RelAggregationJoins {
    // Joins necessary to perform the relation aggregations
    pub(crate) joins: Vec<AliasedJoin>,
    // Aggregator columns
    pub(crate) columns: Vec<Expression<'static>>,
}

pub fn build(ctx: &mut QueryBuilderContext, aggr_selections: &[RelAggregationSelection]) -> RelAggregationJoins {
    let mut joins = vec![];
    let mut columns: Vec<Expression<'static>> = vec![];

    for selection in aggr_selections.iter() {
        match selection {
            RelAggregationSelection::Count(rf, filter) => {
                let (aggregator_alias, join) =
                    ctx.join_builder()
                        .compute_aggr_join(rf, AggregationType::Count, filter.clone(), None);

                columns.push(Column::from((join.alias.clone(), aggregator_alias)).into());
                joins.push(join);
            }
        }
    }

    RelAggregationJoins { joins, columns }
}
