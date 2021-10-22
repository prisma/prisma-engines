use crate::join_utils::{compute_aggr_join, AggregationType, AliasedJoin};
use connector_interface::RelAggregationSelection;
use quaint::prelude::*;

#[derive(Debug)]
pub struct RelAggregationJoins {
    // Joins necessary to perform the relation aggregations
    pub(crate) joins: Vec<AliasedJoin>,
    // Aggregator columns
    pub(crate) columns: Vec<Expression<'static>>,
}

pub fn build(aggr_selections: &[RelAggregationSelection]) -> RelAggregationJoins {
    let mut joins = vec![];
    let mut columns: Vec<Expression<'static>> = vec![];

    for (index, selection) in aggr_selections.iter().enumerate() {
        match selection {
            RelAggregationSelection::Count(rf) => {
                let join_alias = format!("aggr_selection_{}", index);
                let aggregator_alias = selection.db_alias();
                let join = compute_aggr_join(
                    rf,
                    AggregationType::Count,
                    aggregator_alias.as_str(),
                    join_alias.as_str(),
                    None,
                );

                columns.push(Column::from((join.alias.clone(), aggregator_alias)).into());
                joins.push(join);
            }
        }
    }

    RelAggregationJoins { joins, columns }
}
