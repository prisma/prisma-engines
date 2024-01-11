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

// TODO: forward an iterator all the way from `FieldSelection::virtuals` to here without collecting
pub(crate) fn build(virtual_selections: &[&VirtualSelection], ctx: &Context<'_>) -> RelAggregationJoins {
    let mut joins = vec![];
    let mut columns: Vec<Expression<'static>> = vec![];

    for (index, selection) in virtual_selections.iter().enumerate() {
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

                columns.push(Column::from((join.alias.clone(), aggregator_alias)).into());
                joins.push(join);
            }
        }
    }

    RelAggregationJoins { joins, columns }
}
