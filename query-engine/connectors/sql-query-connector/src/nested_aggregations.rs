use crate::{
    join_utils::{AggregationType, AliasedJoin, JoinType},
    query_builder::QueryBuilderContext,
};
use connector_interface::RelAggregationSelection;
use quaint::prelude::*;

#[derive(Debug)]
pub struct RelAggregationColumns {
    // Aggregator columns
    pub(crate) columns: Vec<Expression<'static>>,
}

pub fn build(ctx: &mut QueryBuilderContext, aggr_selections: &[RelAggregationSelection]) -> RelAggregationColumns {
    todo!()
    // let mut columns: Vec<Expression<'static>> = vec![];

    // for selection in aggr_selections.iter() {
    //     match selection {
    //         RelAggregationSelection::Count(rf, filter) => {
    //             let join = ctx.joins().last(&[rf.clone()], JoinType::Aggregation);

    //             todo!()
    //             // columns.push(Column::from((join.alias.clone(), aggregator_alias)).into());
    //             // joins.push(join);
    //         }
    //     }
    // }

    // RelAggregationColumns { joins, columns }
}
