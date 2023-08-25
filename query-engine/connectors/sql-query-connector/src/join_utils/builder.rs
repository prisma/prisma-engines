use super::*;
use crate::{context::Context, filter_conversion::*, model_extensions::*};
use connector_interface::Filter;
use prisma_models::RelationFieldRef;

#[derive(Debug, Default)]
pub(super) struct JoinBuilder {
    join_counter: usize,
    aggregation_counter: usize,
}

impl JoinBuilder {
    pub(crate) fn compute_aggr_join(
        &mut self,
        rf: &RelationFieldRef,
        aggregation: JoinAggregationType,
        filter: Option<&Filter>,
        parent: Option<&AliasedJoin>,
        ctx: &Context<'_>,
    ) -> AliasedJoin {
        if rf.relation().is_many_to_many() {
            self.compute_aggr_join_m2m(rf, aggregation, filter, parent, ctx)
        } else {
            self.compute_aggr_join_one2m(rf, aggregation, filter, parent, ctx)
        }
    }

    pub(crate) fn compute_join(
        &mut self,
        rf: &RelationFieldRef,
        parent: Option<&AliasedJoin>,
        ctx: &Context<'_>,
    ) -> Vec<AliasedJoin> {
        if rf.relation().is_many_to_many() {
            self.compute_m2m_join(rf, parent, ctx)
        } else {
            let join = self.compute_one2m_join(rf, parent, ctx);

            vec![join]
        }
    }

    fn compute_one2m_join(
        &mut self,
        rf: &RelationFieldRef,
        parent: Option<&AliasedJoin>,
        ctx: &Context<'_>,
    ) -> AliasedJoin {
        let join_alias = self.next_alias();

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
                let a_col = match parent {
                    Some(prev_join) => Column::from((prev_join.alias.to_owned(), a.db_name().to_owned())),
                    None => a.as_column(ctx),
                };

                let b_col = Column::from((join_alias.clone(), b.db_name().to_owned()));

                a_col.equals(b_col).into()
            })
            .collect::<Vec<_>>();

        let join = AliasedJoin {
            data: Join::Left(
                related_model
                    .as_table(ctx)
                    .alias(join_alias.clone())
                    .on(ConditionTree::And(on_conditions)),
            ),
            alias: join_alias,
            aggregator_alias: None,
        };
        join
    }

    fn compute_m2m_join(
        &mut self,
        rf: &RelationFieldRef,
        _parent: Option<&AliasedJoin>,
        ctx: &Context<'_>,
    ) -> Vec<AliasedJoin> {
        // First join - parent to m2m join
        let join_alias = self.next_alias();
        let parent_ids: ModelProjection = rf.model().primary_identifier().into();
        let left_join_conditions: Vec<Expression> = parent_ids
            .as_columns(ctx)
            .map(|col_a| {
                let col_b: Vec<_> = rf
                    .related_field()
                    .m2m_columns(ctx)
                    .into_iter()
                    .map(|c| c.table(join_alias.clone()))
                    .collect();

                col_a.equals(col_b).into()
            })
            .collect();
        let parent_to_m2m_join = AliasedJoin {
            data: Join::Left(
                rf.as_table(ctx)
                    .alias(join_alias.clone())
                    .on(ConditionTree::And(left_join_conditions)),
            ),
            alias: join_alias,
            aggregator_alias: None,
        };

        // Second join - m2m to child join
        let join_alias_2 = self.next_alias();
        let child_ids: ModelProjection = rf.related_model().primary_identifier().into();
        let left_join_conditions: Vec<Expression> = child_ids
            .into_iter()
            .map(|c| {
                let col_a = Column::from((join_alias_2.clone(), c.db_name().to_owned()));
                let col_b: Vec<_> = rf
                    .m2m_columns(ctx)
                    .into_iter()
                    .map(|c| c.table(parent_to_m2m_join.alias.clone()))
                    .collect();

                col_a.equals(col_b).into()
            })
            .collect();

        let m2m_to_child_join = AliasedJoin {
            data: Join::Left(
                rf.related_model()
                    .as_table(ctx)
                    .alias(join_alias_2.clone())
                    .on(ConditionTree::And(left_join_conditions)),
            ),
            alias: join_alias_2.clone(),
            aggregator_alias: None,
        };

        vec![parent_to_m2m_join, m2m_to_child_join]
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
        &mut self,
        rf: &RelationFieldRef,
        aggregation: JoinAggregationType,
        filter: Option<&Filter>,
        previous_join: Option<&AliasedJoin>,
        ctx: &Context<'_>,
    ) -> AliasedJoin {
        let join_alias = self.next_alias();
        let aggregator_alias = self.next_aggregator_alias();

        let (left_fields, right_fields) = if rf.is_inlined_on_enclosing_model() {
            (rf.scalar_fields(), rf.referenced_fields())
        } else {
            (
                rf.related_field().referenced_fields(),
                rf.related_field().scalar_fields(),
            )
        };
        let select_columns = right_fields.iter().map(|f| f.as_column(ctx));
        let conditions: ConditionTree = filter
            .map(|f| f.aliased_condition_from(None, false, ctx))
            .unwrap_or(ConditionTree::NoCondition);

        // + SELECT Child.<fk> FROM Child WHERE <FILTER>
        let query = Select::from_table(rf.related_model().as_table(ctx))
            .columns(select_columns)
            .so_that(conditions);
        let aggr_expr = match aggregation {
            JoinAggregationType::Count => count(asterisk()),
        };

        // SELECT Child.<fk>,
        // + COUNT(*) AS <AGGREGATOR_ALIAS>
        // FROM Child WHERE <FILTER>
        let query = query.value(aggr_expr.alias(aggregator_alias.to_owned()));

        // SELECT Child.<fk>, COUNT(*) AS <AGGREGATOR_ALIAS> FROM Child WHERE <FILTER>
        // + GROUP BY Child.<fk>
        let query = right_fields.iter().fold(query, |acc, f| acc.group_by(f.as_column(ctx)));

        let pairs = left_fields.into_iter().zip(right_fields.into_iter());
        let on_conditions: Vec<Expression> = pairs
            .map(|(a, b)| {
                let col_a = match previous_join {
                    Some(prev_join) => Column::from((prev_join.alias.to_owned(), a.db_name().to_owned())),
                    None => a.as_column(ctx),
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
            data: Join::Left(join),
            alias: join_alias.to_owned(),
            aggregator_alias: Some(aggregator_alias),
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
        &mut self,
        rf: &RelationFieldRef,
        aggregation: JoinAggregationType,
        filter: Option<&Filter>,
        previous_join: Option<&AliasedJoin>,
        ctx: &Context<'_>,
    ) -> AliasedJoin {
        let join_alias = self.next_alias();
        let aggregator_alias = self.next_aggregator_alias();

        // m2m join table (_ParentToChild)
        let m2m_table = rf.as_table(ctx);
        // Child colums on the m2m join table (_ParentToChild.ChildId)
        let m2m_child_columns = rf.related_field().m2m_columns(ctx);
        // Child table
        let child_model = rf.related_model();
        // Child primary identifiers
        let child_ids: ModelProjection = rf.related_model().primary_identifier().into();
        // Parent primary identifiers
        let parent_ids: ModelProjection = rf.model().primary_identifier().into();
        // Rendered filters
        let conditions: ConditionTree = filter
            .map(|f| f.aliased_condition_from(None, false, ctx))
            .unwrap_or(ConditionTree::NoCondition);

        // + SELECT _ParentToChild.ChildId FROM Child WHERE <FILTER>
        let query = Select::from_table(child_model.as_table(ctx))
            .columns(m2m_child_columns.clone())
            .so_that(conditions);

        let aggr_expr = match aggregation {
            JoinAggregationType::Count => count(m2m_child_columns.clone()),
        };

        // SELECT _ParentToChild.ChildId,
        // + COUNT(_ParentToChild.ChildId) AS <AGGREGATOR_ALIAS>
        // FROM Child WHERE <FILTER>
        let query = query.value(aggr_expr.alias(aggregator_alias.to_owned()));

        let left_join_conditions: Vec<Expression> = child_ids
            .as_columns(ctx)
            .map(|c| c.equals(rf.m2m_columns(ctx)).into())
            .collect();

        // SELECT _ParentToChild.ChildId, COUNT(_ParentToChild.ChildId) AS <AGGREGATOR_ALIAS> FROM Child WHERE <FILTER>
        // + LEFT JOIN _ParentToChild ON (Child.id = _ParenTtoChild.ChildId)
        let query = query.left_join(m2m_table.on(ConditionTree::And(left_join_conditions)));

        // SELECT _ParentToChild.ChildId, COUNT(_ParentToChild.ChildId) AS <AGGREGATOR_ALIAS> FROM Child WHERE <FILTER>
        // LEFT JOIN _ParentToChild ON (Child.id = _ParentToChild.ChildId)
        // + GROUP BY _ParentToChild.ChildId
        let query = rf
            .related_field()
            .m2m_columns(ctx)
            .into_iter()
            .fold(query, |acc, f| acc.group_by(f.clone()));

        let (left_fields, right_fields) = (parent_ids.scalar_fields(), m2m_child_columns);
        let pairs = left_fields.zip(right_fields);
        let on_conditions: Vec<Expression> = pairs
            .map(|(a, b)| {
                let col_a = match previous_join {
                    Some(prev_join) => Column::from((prev_join.alias.to_owned(), a.db_name().to_owned())),
                    None => a.as_column(ctx),
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
            data: Join::Left(join),
            alias: join_alias.to_owned(),
            aggregator_alias: Some(aggregator_alias),
        }
    }

    fn next_alias(&mut self) -> String {
        self.join_counter += 1;
        let alias = format!("j{}", self.join_counter);

        alias
    }

    fn next_aggregator_alias(&mut self) -> String {
        self.aggregation_counter += 1;
        let alias = format!("aggr{}", self.aggregation_counter);

        alias
    }
}
