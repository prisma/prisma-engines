use crate::context::Context;

use super::{builder::JoinBuilder, *};

use connector_interface::*;
use indexmap::IndexMap;
use prisma_models::*;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct IRJoinStage {
    pub source: RelationFieldRef,
    pub typ: JoinType,
    pub filter: Option<Filter>,
}

type IRJoinSelectionAliases = Option<Vec<String>>;

/// This is an intermediary representation of all joins necessary in order to render a query.
/// Since joins are fed from multiple sources (order bys, relational aggregations, filters...) and since joins can overlap,
/// they're all independently added into a single recursive map which serves as a way to ensure we're eventually rendering a single join to traverse the same relation.
/// The guarantee to render a single join for a same relation makes it much easier to deal with join aliases and potential collisions.
///
/// This IR is eventually converted to a `JoinsContext` which is used throughout the SQL rendering pipeline.
///
/// Note: Each join is currently associated with a list of selection aliases. Atm, this is only used for relational aggregations, which are extracted out of the result sets independently.
#[derive(Debug, Default, Clone)]
pub struct IRJoinsTree {
    inner: IndexMap<IRJoinStage, (IRJoinSelectionAliases, IRJoinsTree)>,
}

impl IRJoinsTree {
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds the necessary joins to perform the order bys.
    pub fn with_order_by(mut self, order_bys: &[OrderBy]) -> Self {
        for o in order_bys {
            match o {
                OrderBy::Scalar(o) => {
                    self.add_nested_joins(&o.relation_hops(), JoinType::Left);
                }
                OrderBy::ToManyAggregation(o) => {
                    if let Some((last, rest)) = o.relation_hops().split_last() {
                        let joins = self.add_nested_joins(rest, JoinType::Left);
                        let join_type = JoinType::Aggregation(JoinAggregationType::from(o.sort_aggregation));

                        joins.add_join(last, join_type);
                    }
                }
                OrderBy::ScalarAggregation(_) => (),
                OrderBy::Relevance(_) => (),
            }
        }

        self
    }

    /// Adds the necessary joins to perform the relation aggregation selections.
    pub fn with_rel_aggregation_selections(mut self, aggr_selections: &[RelAggregationSelection]) -> Self {
        for sel in aggr_selections {
            match sel {
                RelAggregationSelection::Count(rel, filter) => {
                    self.add_join_with_selection(
                        rel,
                        filter.clone(),
                        JoinType::Aggregation(JoinAggregationType::Count),
                        sel.db_alias(),
                    );
                }
            }
        }

        self
    }

    fn add_nested_joins(&mut self, hops: &[&RelationFieldRef], typ: JoinType) -> &mut Self {
        match hops.split_first() {
            Some((first, rest)) => {
                let nested = self.add_join(first, typ);
                nested.add_nested_joins(rest, typ)
            }
            None => self,
        }
    }

    /// Adds a join for a relation.
    fn add_join(&mut self, rf: &RelationFieldRef, typ: JoinType) -> &mut Self {
        let stage = IRJoinStage {
            source: rf.clone(),
            typ,
            filter: None,
        };

        let (_, tree) = self.inner.entry(stage).or_default();

        tree
    }

    /// Adds a join for a relation and the alias to use for selection.
    fn add_join_with_selection(
        &mut self,
        rf: &RelationFieldRef,
        filter: Option<Filter>,
        typ: JoinType,
        selection_alias: String,
    ) -> &mut (IRJoinSelectionAliases, Self) {
        let stage = IRJoinStage {
            source: rf.clone(),
            typ,
            filter,
        };

        match self.inner.entry(stage) {
            // If the join already exists, push the selection alias to the list.
            indexmap::map::Entry::Occupied(entry) => {
                let value = entry.into_mut();

                match value.0 {
                    Some(ref mut selections) => selections.push(selection_alias),
                    None => value.0 = Some(vec![selection_alias]),
                };

                value
            }
            indexmap::map::Entry::Vacant(entry) => entry.insert((Some(vec![selection_alias]), Self::default())),
        }
    }

    pub(crate) fn build(self, ctx: &Context<'_>) -> JoinsContext {
        let mut builder = JoinBuilder::default();

        self.build_internal(&mut builder, None, ctx)
    }

    fn build_internal(
        self,
        builder: &mut JoinBuilder,
        parent: Option<&AliasedJoin>,
        ctx: &Context<'_>,
    ) -> JoinsContext {
        let joins = self
            .inner
            .into_iter()
            .map(|(k, (selections, nested_joins))| match k.typ {
                JoinType::Aggregation(JoinAggregationType::Count) => {
                    let join = builder.compute_aggr_join(
                        &k.source,
                        JoinAggregationType::Count,
                        k.filter.as_ref(),
                        parent,
                        ctx,
                    );

                    let selections = selections.map(|selections| {
                        selections
                            .into_iter()
                            .map(|selection| {
                                col!(join.alias.clone(), join.aggregator_alias.as_ref().unwrap().clone())
                                    .alias(selection)
                            })
                            .collect()
                    });

                    let nested = nested_joins.build_internal(builder, Some(&join), ctx);

                    JoinStage {
                        meta: k,
                        data: vec![join],
                        nested,
                        selections,
                    }
                }
                _ => {
                    let joins = builder.compute_join(&k.source, parent, ctx);
                    let nested = nested_joins.build_internal(builder, joins.last(), ctx);

                    JoinStage {
                        meta: k,
                        data: joins,
                        nested,
                        selections: None,
                    }
                }
            })
            .collect();

        JoinsContext { joins }
    }
}
