pub mod alias;
mod visitor;

use quaint::prelude::*;
use query_structure::Filter;
pub use visitor::*;

use crate::{context::Context, join_utils::AliasedJoin};

pub(crate) struct FilterBuilder {}
pub(crate) struct FilterBuilderWithJoins {}
pub(crate) struct FilterBuilderWithoutJoins {}

impl FilterBuilder {
    pub(crate) fn with_top_level_joins() -> FilterBuilderWithJoins {
        FilterBuilderWithJoins {}
    }

    pub(crate) fn without_top_level_joins() -> FilterBuilderWithoutJoins {
        FilterBuilderWithoutJoins {}
    }
}

impl FilterBuilderWithJoins {
    /// Visits a filter and return additional top-level joins that need to be manually dealt with.
    pub(crate) fn visit_filter(
        &self,
        filter: Filter,
        ctx: &Context,
    ) -> (ConditionTree<'static>, Option<Vec<AliasedJoin>>) {
        FilterVisitor::with_top_level_joins().visit_filter(filter, ctx)
    }
}

impl FilterBuilderWithoutJoins {
    /// Visits a filter without any top-level joins. Can be safely used in any context.
    pub(crate) fn visit_filter(&self, filter: Filter, ctx: &Context) -> ConditionTree<'static> {
        let (cond, _) = FilterVisitor::without_top_level_joins().visit_filter(filter, ctx);

        cond
    }
}
