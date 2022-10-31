use crate::{filter_conversion::AliasedCondition, model_extensions::*};
use connector_interface::{Filter, RelationCondition, RelationFilter};
use prisma_models::*;
use quaint::prelude::*;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct AliasedJoin {
    // Actual join data to be passed to quaint
    pub(crate) data: JoinData<'static>,
    // Alias used for the join. eg: LEFT JOIN ... AS <alias>
    pub(crate) alias: String,
}

#[derive(Debug, Default)]
pub struct JoinBuilder {
    count: usize,
    cache: HashMap<RelationFieldRef, AliasedJoin>,
    m2m_cache: HashMap<RelationFieldRef, Vec<AliasedJoin>>,
    aggr_cache: HashMap<RelationFieldRef, (String, AliasedJoin)>,
}

impl JoinBuilder {
    pub fn compute_aggr_join(
        &mut self,
        rf: &RelationFieldRef,
        aggregation: AggregationType,
        filter: Option<Filter>,
        previous_join: Option<&AliasedJoin>,
    ) -> (String, AliasedJoin) {
        if let Some(entry) = self.aggr_cache.get(rf) {
            return entry.clone();
        }

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

        self.aggr_cache
            .insert(rf.clone(), (aggregator_alias.clone(), join.clone()));

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
            vec![self.compute_one2m_join(rf, filter, previous_join)]
        }
    }

    fn compute_one2m_join(
        &mut self,
        rf: &RelationFieldRef,
        filter: Option<&Filter>,
        previous_join: Option<&AliasedJoin>,
    ) -> AliasedJoin {
        if let Some(entry) = self.cache.get(rf) {
            return entry.clone();
        }

        dbg!(&filter);
        let conditions: ConditionTree = filter
            .map(|f| {
                Filter::Relation(RelationFilter {
                    field: rf.clone(),
                    condition: RelationCondition::EveryRelatedRecord,
                    nested_filter: Box::new(f.clone()),
                })
            })
            .map(|f| {
                dbg!(&f);

                f.aliased_condition_from(None, false)
            })
            .unwrap_or(ConditionTree::NoCondition);

        let join_alias = self.join_alias();
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
                .on(ConditionTree::And(on_conditions)),
        };

        self.cache.insert(rf.clone(), join.clone());

        join
    }

    fn compute_m2m_join(
        &mut self,
        rf: &RelationFieldRef,
        filter: Option<&Filter>,
        previous_join: Option<&AliasedJoin>,
    ) -> Vec<AliasedJoin> {
        if let Some(entry) = self.m2m_cache.get(rf) {
            return entry.clone();
        }

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
                .on(ConditionTree::And(left_join_conditions)),
        };

        let joins = vec![parent_to_m2m_join, m2m_to_child_join];

        self.m2m_cache.insert(rf.clone(), joins.clone());

        joins
    }

    fn join_alias(&mut self) -> String {
        let alias = format!("j{}", self.count);
        self.count += 1;

        alias
    }

    fn aggregator_alias(&mut self) -> String {
        let alias = format!("aggr{}", self.count);
        self.count += 1;

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
