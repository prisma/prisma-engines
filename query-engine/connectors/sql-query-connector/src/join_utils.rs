use crate::{
    filter_conversion::{Alias, AliasMode, AliasedCondition},
    model_extensions::*,
};
use connector_interface::{Filter, NestedRead};
use itertools::Itertools;
use prisma_models::*;
use quaint::prelude::*;
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum JoinType {
    Normal,
    Aggregation,
}

type InnerJoins = HashMap<(RelationFieldRef, JoinType), (Vec<AliasedJoin>, JoinsMap)>;

#[derive(Debug, Clone, Default)]
pub struct JoinsMap {
    inner: InnerJoins,
}

impl JoinsMap {
    pub fn new(nested_reads: &[NestedRead], order_bys: &[OrderBy]) -> Self {
        let internal_joins = InternalJoins::new(nested_reads, order_bys);
        let mut builder = JoinBuilder::default();

        Self::build(&mut builder, internal_joins, None)
    }

    pub fn last(&self, relations: &[RelationFieldRef], join_type: JoinType) -> Option<&AliasedJoin> {
        if relations.is_empty() {
            return None;
        }

        // unwrap safe because we know the `relations` array is not empty
        let (first, rest) = relations.split_first().unwrap();
        let first_join = self.get(first, join_type);

        let res = rest
            .iter()
            .fold(first_join, |acc, rf| {
                if let Some((_, nested)) = acc {
                    nested.get(rf, join_type)
                } else {
                    panic!("could not find join for relation {:?}", rf);
                }
            })
            .and_then(|(joins, _)| joins.last());

        res
    }

    pub fn all(&self, relations: &[RelationFieldRef], join_type: JoinType) -> Option<Vec<&AliasedJoin>> {
        if relations.is_empty() {
            return None;
        }

        let mut res = vec![];
        let mut joins_map = self;

        for rf in relations {
            if let Some((joins, nested_map)) = joins_map.get(rf, join_type) {
                res.extend(joins);
                joins_map = nested_map;
            } else {
                panic!("could not find join for relation {:?}", rf);
            }
        }

        Some(res)
    }

    pub fn to_vec(self) -> Vec<AliasedJoin> {
        let mut all_joins = vec![];

        for (joins, nested) in self.inner.into_values() {
            all_joins.extend(joins);
            all_joins.extend(nested.to_vec());
        }

        all_joins
    }

    fn get(&self, rf: &RelationFieldRef, join_type: JoinType) -> Option<&(Vec<AliasedJoin>, JoinsMap)> {
        self.inner.get(&(rf.clone(), join_type))
    }

    fn build(builder: &mut JoinBuilder, joins_data: InternalJoins, previous_join: Option<&AliasedJoin>) -> Self {
        let mut map: InnerJoins = HashMap::new();

        for ((rf, join_type), (filter, nested_joins)) in joins_data.inner.into_iter() {
            match join_type {
                JoinType::Normal => {
                    let computed_joins = builder.compute_join(&rf, filter.as_ref(), previous_join);
                    let nested_joins = Self::build(builder, nested_joins, computed_joins.last());

                    map.insert((rf, join_type.clone()), (computed_joins, nested_joins));
                }
                JoinType::Aggregation => todo!(),
            }
        }

        Self { inner: map }
    }
}

#[derive(Debug, Clone, Default)]
struct InternalJoins {
    inner: HashMap<(RelationFieldRef, JoinType), (Option<Filter>, InternalJoins)>,
}

impl InternalJoins {
    pub fn new(nested_reads: &[NestedRead], order_bys: &[OrderBy]) -> InternalJoins {
        let joins = Self::from_nested_reads(nested_reads);
        let joins = Self::extend_with_order_bys(joins, order_bys);

        joins
    }

    fn from_nested_reads(nested_reads: &[NestedRead]) -> InternalJoins {
        let map: HashMap<(RelationFieldRef, JoinType), (Option<Filter>, InternalJoins)> = nested_reads
            .iter()
            .map(|read| {
                let filter = read.args.filter.clone();

                (
                    (read.parent_field.clone(), JoinType::Normal),
                    (filter, Self::from_nested_reads(&read.nested)),
                )
            })
            .collect();

        InternalJoins { inner: map }
    }

    fn extend_with_order_bys(mut joins: InternalJoins, order_bys: &[OrderBy]) -> InternalJoins {
        for o in order_bys {
            match o {
                OrderBy::Scalar(o) => {
                    let path = o
                        .path
                        .iter()
                        .filter_map(|hop| hop.as_relation_hop().cloned())
                        .collect_vec();

                    path.iter().fold(&mut joins, |acc, next| {
                        let rf = next.clone();

                        let (_, nested_joins) = acc
                            .inner
                            .entry((rf, JoinType::Normal))
                            .or_insert_with(|| (None, InternalJoins::default()));

                        nested_joins
                    });
                }
                OrderBy::ToManyAggregation(o) => {
                    let path: Vec<_> = o
                        .path
                        .iter()
                        .filter_map(|hop| hop.as_relation_hop().cloned())
                        .collect_vec();

                    let (last, rest) = path.split_last().unwrap();

                    let inner_joins = rest.iter().fold(&mut joins, |acc, next| {
                        let rf = next.clone();

                        let (_, nested_joins) = acc
                            .inner
                            .entry((rf, JoinType::Aggregation))
                            .or_insert_with(|| (None, InternalJoins::default()));

                        nested_joins
                    });

                    inner_joins
                        .inner
                        .entry((last.clone(), JoinType::Aggregation))
                        .or_insert_with(|| (None, InternalJoins::default()));
                }
                OrderBy::ScalarAggregation(_) => (),
                OrderBy::Relevance(_) => (),
            }
        }

        joins
    }
}

#[derive(Debug, Clone)]
pub struct AliasedJoin {
    // Actual join data to be passed to quaint
    pub(crate) data: JoinData<'static>,
    // Alias used for the join. eg: LEFT JOIN ... AS <alias>
    pub(crate) alias: String,
}

#[derive(Debug, Default)]
pub struct JoinBuilder {
    counter: usize,
}

impl JoinBuilder {
    pub fn compute_aggr_join(
        &mut self,
        rf: &RelationFieldRef,
        aggregation: AggregationType,
        filter: Option<Filter>,
        previous_join: Option<&AliasedJoin>,
    ) -> (String, AliasedJoin) {
        // if let Some(entry) = self.aggr_cache.get(rf) {
        //     return entry.clone();
        // }

        let join_alias = self.join_alias();
        let aggregator_alias = self.aggregator_alias();

        let join = if rf.relation().is_many_to_many() {
            compute_aggr_join_m2m(
                rf,
                aggregation,
                filter,
                aggregator_alias.as_str(),
                join_alias.as_str(),
                previous_join,
            )
        } else {
            compute_aggr_join_one2m(
                rf,
                aggregation,
                filter,
                aggregator_alias.as_str(),
                join_alias.as_str(),
                previous_join,
            )
        };

        // self.aggr_cache
        //     .insert(rf.clone(), (aggregator_alias.clone(), join.clone()));

        (aggregator_alias, join)
    }

    pub fn compute_join(
        &mut self,
        rf: &RelationFieldRef,
        filter: Option<&Filter>,
        previous_join: Option<&AliasedJoin>,
    ) -> Vec<AliasedJoin> {
        if rf.relation().is_many_to_many() {
            self.compute_m2m_join(rf, filter, previous_join)
        } else {
            let join = self.compute_one2m_join(rf, filter, previous_join);

            vec![join]
        }
    }

    fn compute_one2m_join(
        &mut self,
        rf: &RelationFieldRef,
        filter: Option<&Filter>,
        previous_join: Option<&AliasedJoin>,
    ) -> AliasedJoin {
        // if let Some(entry) = self.one2m_cache.get(rf) {
        //     return (entry.clone(), true);
        // }

        let join_alias = self.join_alias();

        let filter: ConditionTree = filter
            .map(|f| {
                let mut alias = Alias::default().flip(AliasMode::Join);
                alias.set_counter(self.counter - 1);

                f.aliased_condition_from(Some(alias), false)
            })
            .unwrap_or(ConditionTree::NoCondition);

        let (left_fields, right_fields) = if rf.is_inlined_on_enclosing_model() {
            (rf.scalar_fields(), rf.referenced_fields())
        } else {
            (
                rf.related_field().referenced_fields(),
                rf.related_field().scalar_fields(),
            )
        };

        let related_model = rf.related_model();
        let pairs = left_fields.into_iter().zip(right_fields.into_iter());

        let on_conditions: Vec<Expression> = pairs
            .map(|(a, b)| {
                let a_col = match previous_join {
                    Some(prev_join) => Column::from((prev_join.alias.to_owned(), a.db_name().to_owned())),
                    None => a.as_column(),
                };

                let b_col = Column::from((join_alias.clone(), b.db_name().to_owned()));

                a_col.equals(b_col).into()
            })
            .collect::<Vec<_>>();

        let join = AliasedJoin {
            alias: join_alias.clone(),
            data: related_model
                .as_table()
                .alias(join_alias)
                .on(ConditionTree::And(on_conditions).and(filter)),
        };

        // self.one2m_cache.insert(rf.clone(), join.clone());

        join
    }

    fn compute_m2m_join(
        &mut self,
        rf: &RelationFieldRef,
        filter: Option<&Filter>,
        _previous_join: Option<&AliasedJoin>,
    ) -> Vec<AliasedJoin> {
        // if let Some(entry) = self.m2m_cache.get(rf) {
        //     return (entry.clone(), true);
        // }

        // First join - parent to m2m join
        let join_alias = self.join_alias();
        let parent_ids: ModelProjection = rf.model().primary_identifier().into();
        let left_join_conditions: Vec<Expression> = parent_ids
            .as_columns()
            .into_iter()
            .map(|col_a| {
                let col_b: Vec<_> = rf
                    .related_field()
                    .m2m_columns()
                    .into_iter()
                    .map(|c| c.table(join_alias.clone()))
                    .collect();

                col_a.equals(col_b).into()
            })
            .collect();
        let parent_to_m2m_join = AliasedJoin {
            alias: join_alias.clone(),
            data: rf
                .as_table()
                .alias(join_alias)
                .on(ConditionTree::And(left_join_conditions)),
        };

        // Second join - m2m to child join
        let join_alias_2 = self.join_alias();
        let filter: ConditionTree = filter
            .map(|f| {
                let mut alias = Alias::default().flip(AliasMode::Join);
                alias.set_counter(self.counter - 1);

                f.aliased_condition_from(Some(alias), false)
            })
            .unwrap_or(ConditionTree::NoCondition);
        let child_ids: ModelProjection = rf.related_model().primary_identifier().into();
        let left_join_conditions: Vec<Expression> = child_ids
            .into_iter()
            .map(|c| {
                let col_a = Column::from((join_alias_2.clone(), c.db_name().to_owned()));
                let col_b: Vec<_> = rf
                    .m2m_columns()
                    .into_iter()
                    .map(|c| c.table(parent_to_m2m_join.alias.clone()))
                    .collect();

                col_a.equals(col_b).into()
            })
            .collect();
        let m2m_to_child_join = AliasedJoin {
            alias: join_alias_2.clone(),
            data: rf
                .related_model()
                .as_table()
                .alias(join_alias_2.clone())
                .on(ConditionTree::And(left_join_conditions).and(filter)),
        };

        let joins = vec![parent_to_m2m_join, m2m_to_child_join];

        // self.m2m_cache.insert(rf.clone(), joins.clone());

        joins
    }

    fn join_alias(&mut self) -> String {
        let alias = format!("j{}", self.counter);
        self.counter += 1;

        alias
    }

    fn aggregator_alias(&mut self) -> String {
        let alias = format!("aggr{}", self.counter);
        self.counter += 1;

        alias
    }
}

#[derive(Debug, Clone)]
pub enum AggregationType {
    Count,
}

/// Computes a one-to-many join for an aggregation (in aggregation selections, order by...).
///
/// Preview of the rendered SQL:
/// ```sql
/// LEFT JOIN (
///     SELECT Child.<fk>, COUNT(*) AS <AGGREGATOR_ALIAS> FROM Child WHERE <filter>
///     GROUP BY Child.<fk>
/// ) AS <ORDER_JOIN_PREFIX> ON (<Parent | previous_join_alias>.<fk> = <ORDER_JOIN_PREFIX>.<fk>)
/// ```
fn compute_aggr_join_one2m(
    rf: &RelationFieldRef,
    aggregation: AggregationType,
    filter: Option<Filter>,
    aggregator_alias: &str,
    join_alias: &str,
    previous_join: Option<&AliasedJoin>,
) -> AliasedJoin {
    let (left_fields, right_fields) = if rf.is_inlined_on_enclosing_model() {
        (rf.scalar_fields(), rf.referenced_fields())
    } else {
        (
            rf.related_field().referenced_fields(),
            rf.related_field().scalar_fields(),
        )
    };
    let select_columns = right_fields.iter().map(|f| f.as_column());
    let conditions: ConditionTree = filter
        .map(|f| f.aliased_condition_from(None, false))
        .unwrap_or(ConditionTree::NoCondition);

    // + SELECT Child.<fk> FROM Child WHERE <FILTER>
    let query = Select::from_table(rf.related_model().as_table())
        .columns(select_columns)
        .so_that(conditions);
    let aggr_expr = match aggregation {
        AggregationType::Count => count(asterisk()),
    };

    // SELECT Child.<fk>,
    // + COUNT(*) AS <AGGREGATOR_ALIAS>
    // FROM Child WHERE <FILTER>
    let query = query.value(aggr_expr.alias(aggregator_alias.to_owned()));

    // SELECT Child.<fk>, COUNT(*) AS <AGGREGATOR_ALIAS> FROM Child WHERE <FILTER>
    // + GROUP BY Child.<fk>
    let query = right_fields.iter().fold(query, |acc, f| acc.group_by(f.as_column()));

    let pairs = left_fields.into_iter().zip(right_fields.into_iter());
    let on_conditions: Vec<Expression> = pairs
        .map(|(a, b)| {
            let col_a = match previous_join {
                Some(prev_join) => Column::from((prev_join.alias.to_owned(), a.db_name().to_owned())),
                None => a.as_column(),
            };
            let col_b = Column::from((join_alias.to_owned(), b.db_name().to_owned()));

            col_a.equals(col_b).into()
        })
        .collect::<Vec<_>>();

    // + LEFT JOIN (
    //     SELECT Child.<fk>, COUNT(*) AS <AGGREGATOR_ALIAS> FROM Child WHERE <FILTER>
    //     GROUP BY Child.<fk>
    // + ) AS <ORDER_JOIN_PREFIX> ON (<Parent | previous_join_alias>.<fk> = <ORDER_JOIN_PREFIX>.<fk>)
    let join = Table::from(query)
        .alias(join_alias.to_owned())
        .on(ConditionTree::And(on_conditions));

    AliasedJoin {
        data: join,
        alias: join_alias.to_owned(),
    }
}

/// Compoutes a many-to-many join for an aggregation (in aggregation selections, order by...).
///
/// Preview of the rendered SQL:
/// ```sql
/// LEFT JOIN (
///   SELECT _ParentToChild.ChildId, COUNT(_ParentToChild.ChildId) AS <AGGREGATOR_ALIAS> FROM Child WHERE <FILTER>
///   LEFT JOIN _ParentToChild ON (Child.id = _ParentToChild.ChildId)
///   GROUP BY _ParentToChild.ChildId
/// ) AS <ORDER_JOIN_PREFIX> ON (<Parent | previous_join_alias>.id = <ORDER_JOIN_PREFIX>.ChildId)
/// ```
fn compute_aggr_join_m2m(
    rf: &RelationFieldRef,
    aggregation: AggregationType,
    filter: Option<Filter>,
    aggregator_alias: &str,
    join_alias: &str,
    previous_join: Option<&AliasedJoin>,
) -> AliasedJoin {
    // m2m join table (_ParentToChild)
    let m2m_table = rf.as_table();
    // Child colums on the m2m join table (_ParentToChild.ChildId)
    let m2m_child_columns = rf.related_field().m2m_columns();
    // Child table
    let child_model = rf.related_model();
    // Child primary identifiers
    let child_ids: ModelProjection = rf.related_model().primary_identifier().into();
    // Parent primary identifiers
    let parent_ids: ModelProjection = rf.model().primary_identifier().into();
    // Rendered filters
    let conditions: ConditionTree = filter
        .map(|f| f.aliased_condition_from(None, false))
        .unwrap_or(ConditionTree::NoCondition);

    // + SELECT _ParentToChild.ChildId FROM Child WHERE <FILTER>
    let query = Select::from_table(child_model.as_table())
        .columns(m2m_child_columns.clone())
        .so_that(conditions);

    let aggr_expr = match aggregation {
        AggregationType::Count => count(m2m_child_columns.clone()),
    };

    // SELECT _ParentToChild.ChildId,
    // + COUNT(_ParentToChild.ChildId) AS <AGGREGATOR_ALIAS>
    // FROM Child WHERE <FILTER>
    let query = query.value(aggr_expr.alias(aggregator_alias.to_owned()));

    let left_join_conditions: Vec<Expression> = child_ids
        .as_columns()
        .into_iter()
        .map(|c| c.equals(rf.m2m_columns()).into())
        .collect();

    // SELECT _ParentToChild.ChildId, COUNT(_ParentToChild.ChildId) AS <AGGREGATOR_ALIAS> FROM Child WHERE <FILTER>
    // + LEFT JOIN _ParentToChild ON (Child.id = _ParenTtoChild.ChildId)
    let query = query.left_join(m2m_table.on(ConditionTree::And(left_join_conditions)));

    // SELECT _ParentToChild.ChildId, COUNT(_ParentToChild.ChildId) AS <AGGREGATOR_ALIAS> FROM Child WHERE <FILTER>
    // LEFT JOIN _ParentToChild ON (Child.id = _ParentToChild.ChildId)
    // + GROUP BY _ParentToChild.ChildId
    let query = rf
        .related_field()
        .m2m_columns()
        .into_iter()
        .fold(query, |acc, f| acc.group_by(f.clone()));

    let (left_fields, right_fields) = (parent_ids.scalar_fields(), m2m_child_columns);
    let pairs = left_fields.zip(right_fields);
    let on_conditions: Vec<Expression> = pairs
        .map(|(a, b)| {
            let col_a = match previous_join {
                Some(prev_join) => Column::from((prev_join.alias.to_owned(), a.db_name().to_owned())),
                None => a.as_column(),
            };
            let col_b = Column::from((join_alias.to_owned(), b.name.to_string()));

            col_a.equals(col_b).into()
        })
        .collect::<Vec<_>>();

    // + LEFT JOIN (
    //     SELECT _ParentToChild.ChildId, COUNT(_ParentToChild.ChildId) AS <AGGREGATOR_ALIAS> FROM Child WHERE <FILTER>
    //     LEFT JOIN _ParentToChild ON (Child.id = _ParentToChild.ChildId)
    //     GROUP BY _ParentToChild.ChildId
    // + ) AS <ORDER_JOIN_PREFIX> ON (<Parent | previous_join_alias>.id = <ORDER_JOIN_PREFIX>.ChildId)
    let join = Table::from(query)
        .alias(join_alias.to_owned())
        .on(ConditionTree::And(on_conditions));

    AliasedJoin {
        alias: join_alias.to_owned(),
        data: join,
    }
}
