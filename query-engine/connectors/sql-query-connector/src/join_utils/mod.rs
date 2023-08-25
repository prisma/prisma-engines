mod builder;
mod ir;

pub use ir::*;
use itertools::Itertools;
use prisma_models::*;
use quaint::prelude::*;
use std::{hash::Hash, iter};

#[derive(Debug, Clone)]
pub struct JoinsContext {
    joins: Vec<JoinStage>,
}

#[derive(Debug, Clone)]
pub struct JoinStage {
    pub data: Vec<AliasedJoin>,
    pub selections: Option<Vec<Expression<'static>>>,
    pub meta: IRJoinStage,
    pub nested: JoinsContext,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[allow(dead_code)]
pub enum JoinType {
    Left,
    Right,
    Inner,
    Full,
    Aggregation(JoinAggregationType),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum JoinAggregationType {
    Count,
}

#[derive(Debug, Clone)]
pub struct AliasedJoin {
    /// Actual join data to be passed to quaint
    pub(crate) data: Join<'static>,
    /// Alias used for the join. eg: LEFT JOIN ... AS <alias>
    pub(crate) alias: String,
    /// Aliased used for the aggregation inside the join. eg: LEFT JOIN (SELECT COUNT() AS <aggregator_alias>) AS ...
    pub(crate) aggregator_alias: Option<String>,
}

impl JoinStage {
    pub fn collect_joins(&self) -> Vec<AliasedJoin> {
        let mut res: Vec<AliasedJoin> = self.data.clone();

        for nested_joins in &self.nested.joins {
            res.append(&mut nested_joins.collect_joins());
        }

        res
    }
}

impl JoinsContext {
    pub fn get_from_order_by_scalar(&self, o: &OrderByScalar) -> Option<Vec<&AliasedJoin>> {
        let hops = o
            .relation_hops()
            .into_iter()
            .map(|hop| IRJoinStage {
                source: hop.clone(),
                typ: JoinType::Left,
                filter: None,
            })
            .collect_vec();

        self.get_from_hops(hops.as_slice())
    }

    pub fn get_from_order_by_to_many(&self, o: &OrderByToManyAggregation) -> Option<Vec<&AliasedJoin>> {
        let hops: Vec<_> = o.relation_hops().into_iter().map(Clone::clone).collect();
        let (last, rest) = hops.split_last().unwrap();
        let hops: Vec<_> = rest
            .iter()
            .map(|hop| IRJoinStage {
                source: hop.clone(),
                typ: JoinType::Left,
                filter: None,
            })
            .chain(iter::once(IRJoinStage {
                source: last.clone(),
                typ: JoinType::Aggregation(JoinAggregationType::from(o.sort_aggregation)),
                filter: None,
            }))
            .collect();

        self.get_from_hops(hops.as_slice())
    }

    fn get_from_hops(&self, hops: &[IRJoinStage]) -> Option<Vec<&AliasedJoin>> {
        if hops.is_empty() {
            return Some(Vec::new());
        }

        let (source, rest) = hops.split_first().unwrap();

        let mut joins = vec![];

        if let Some(join_info) = self.find_join(source) {
            match join_info.nested.get_from_hops(rest) {
                Some(data) => {
                    joins.extend(&join_info.data);
                    joins.extend(data);
                }
                None => joins.extend(&join_info.data),
            }
        };

        Some(joins)
    }

    pub fn find_join(&self, stage: &IRJoinStage) -> Option<&JoinStage> {
        self.joins.iter().find(|j| j.meta == *stage)
    }

    pub fn selections(&self) -> Vec<Expression<'static>> {
        let mut columns: Vec<_> = vec![];

        for join in self.joins.iter() {
            if let Some(mut selections) = join.selections.clone() {
                columns.append(&mut selections);
            }
            columns.append(&mut join.nested.selections());
        }

        columns
    }

    pub fn iter_joins(&self) -> impl Iterator<Item = AliasedJoin> + '_ {
        self.joins.iter().flat_map(|j| j.collect_joins())
    }
}

impl From<SortAggregation> for JoinAggregationType {
    fn from(value: SortAggregation) -> Self {
        match value {
            SortAggregation::Count => Self::Count,
            _ => unreachable!(),
        }
    }
}
