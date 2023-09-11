mod alias;
mod visitor;

use connector_interface::Filter;
use quaint::prelude::*;
use visitor::*;

use crate::{context::Context, join_utils::AliasedJoin};

pub(crate) struct FilterBuilder {}

pub(crate) struct FilterBuilderWithJoins {}

impl FilterBuilderWithJoins {
    pub(crate) fn visit_filter(
        &self,
        filter: Filter,
        ctx: &Context,
    ) -> (ConditionTree<'static>, Option<Vec<AliasedJoin>>) {
        FilterVisitor::with_joins().visit_filter(filter, ctx)
    }
}
pub(crate) struct FilterBuilderWithoutJoins {}

impl FilterBuilderWithoutJoins {
    pub(crate) fn visit_filter(&self, filter: Filter, ctx: &Context) -> ConditionTree<'static> {
        let (cond, _) = FilterVisitor::without_joins().visit_filter(filter, ctx);

        cond
    }
}

impl FilterBuilder {
    pub(crate) fn with_joins() -> FilterBuilderWithJoins {
        FilterBuilderWithJoins {}
    }

    pub(crate) fn without_joins() -> FilterBuilderWithoutJoins {
        FilterBuilderWithoutJoins {}
    }
}
