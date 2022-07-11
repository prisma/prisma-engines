use crate::{filter_conversion::AliasedCondition, model_extensions::*};
use connector_interface::Filter;
use prisma_models::*;
use quaint::prelude::*;

#[derive(Debug, Clone)]
pub struct AliasedJoin {
    // Actual join data to be passed to quaint
    pub(crate) data: JoinData<'static>,
    // Alias used for the join. eg: LEFT JOIN ... AS <alias>
    pub(crate) alias: String,
}

#[derive(Debug, Clone)]
pub enum AggregationType {
    Count,
}

pub fn compute_aggr_join(
    rf: &RelationFieldRef,
    aggregation: AggregationType,
    filter: Option<Filter>,
    aggregator_alias: &str,
    join_alias: &str,
    previous_join: Option<&AliasedJoin>,
) -> AliasedJoin {
    let join_alias = format!("{}_{}", join_alias, &rf.related_model().name);

    if rf.relation().is_many_to_many() {
        compute_aggr_join_m2m(
            rf,
            aggregation,
            filter,
            aggregator_alias,
            join_alias.as_str(),
            previous_join,
        )
    } else {
        compute_aggr_join_one2m(
            rf,
            aggregation,
            filter,
            aggregator_alias,
            join_alias.as_str(),
            previous_join,
        )
    }
}

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
        .map(|f| f.aliased_cond(None))
        .unwrap_or(ConditionTree::NoCondition);

    // + SELECT Child.<fk> FROM A
    let query = Select::from_table(rf.related_model().as_table())
        .columns(select_columns)
        .so_that(conditions);
    let aggr_expr = match aggregation {
        AggregationType::Count => count(asterisk()),
    };

    // SELECT Child.<fk>,
    // + COUNT(*) AS <AGGREGATOR_ALIAS>
    // FROM Child
    let query = query.value(aggr_expr.alias(aggregator_alias.to_owned()));

    // SELECT Child.<fk>, COUNT(*) AS <AGGREGATOR_ALIAS> FROM Child
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
    //     SELECT Child.<fk>, COUNT(*) AS <AGGREGATOR_ALIAS> FROM Child
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

fn compute_aggr_join_m2m(
    rf: &RelationFieldRef,
    aggregation: AggregationType,
    filter: Option<Filter>,
    aggregator_alias: &str,
    join_alias: &str,
    previous_join: Option<&AliasedJoin>,
) -> AliasedJoin {
    // _ParentToChild table
    let m2m_table = rf.as_table();
    // _ParentToChild.Child columns
    let m2m_child_columns = rf.related_field().m2m_columns();
    // Child table
    let child_model = rf.related_model();
    // Child primary identifiers
    let child_ids: ModelProjection = rf.related_model().primary_identifier().into();
    // Parent primary identifiers
    let parent_ids: ModelProjection = rf.model().primary_identifier().into();
    // Rendered filters
    let conditions: ConditionTree = filter
        .map(|f| f.aliased_cond(None))
        .unwrap_or(ConditionTree::NoCondition);

    // + SELECT _ParentToChild.Child FROM Child
    let query = Select::from_table(child_model.as_table())
        .columns(m2m_child_columns.clone())
        .so_that(conditions);

    let aggr_expr = match aggregation {
        AggregationType::Count => count(m2m_child_columns.clone()),
    };

    // SELECT _ParentToChild.Child,
    // + COUNT(_ParentToChild.Child) AS <AGGREGATOR_ALIAS>
    // FROM Child
    let query = query.value(aggr_expr.alias(aggregator_alias.to_owned()));

    let left_join_conditions: Vec<Expression> = child_ids
        .as_columns()
        .into_iter()
        .map(|c| c.equals(rf.m2m_columns()).into())
        .collect();

    // SELECT _ParentToChild.Child, COUNT(_ParenTtoChild.Child) AS <AGGREGATOR_ALIAS> FROM Child
    // + LEFT JOIN _ParentToChild ON (B.id = _ParenTtoChild.Child)
    let query = query.left_join(m2m_table.on(ConditionTree::And(left_join_conditions)));

    // SELECT _ParentToChild.Child, COUNT(_ParenttoChild.Child) AS <AGGREGATOR_ALIAS> FROM Child
    // LEFT JOIN _ParentToChild ON (Child.id = _ParenttoChild.Child)
    // + GROUP BY _ParentToChild.Child
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
    //     SELECT _ParentToChild.Child, COUNT(_ParentToChild.Child) AS <AGGREGATOR_ALIAS> FROM Child
    //     LEFT JOIN _ParentToChild ON (B.id = _ParentToChild.Child)
    //     GROUP BY _ParentToChild.Child
    // + ) AS <ORDER_JOIN_PREFIX> ON (<Parent | previous_join_alias>.id = <ORDER_JOIN_PREFIX>.B)
    let join = Table::from(query)
        .alias(join_alias.to_owned())
        .on(ConditionTree::And(on_conditions));

    AliasedJoin {
        alias: join_alias.to_owned(),
        data: join,
    }
}

pub fn compute_one2m_join(base_model: &ModelRef, rf: &RelationFieldRef, join_prefix: &str) -> AliasedJoin {
    let (left_fields, right_fields) = if rf.is_inlined_on_enclosing_model() {
        (rf.scalar_fields(), rf.referenced_fields())
    } else {
        (
            rf.related_field().referenced_fields(),
            rf.related_field().scalar_fields(),
        )
    };

    // `rf` is always the relation field on the left model in the join (parent).
    let left_table_alias = if rf.model().name != base_model.name {
        Some(format!("{}_{}", join_prefix, &rf.model().name))
    } else {
        None
    };

    let right_table_alias = format!("{}_{}", join_prefix, &rf.related_model().name);

    let related_model = rf.related_model();
    let pairs = left_fields.into_iter().zip(right_fields.into_iter());

    let on_conditions: Vec<Expression> = pairs
        .map(|(a, b)| {
            let a_col = if let Some(alias) = left_table_alias.clone() {
                Column::from((alias, a.db_name().to_owned()))
            } else {
                a.as_column()
            };

            let b_col = Column::from((right_table_alias.clone(), b.db_name().to_owned()));

            a_col.equals(b_col).into()
        })
        .collect::<Vec<_>>();

    AliasedJoin {
        alias: right_table_alias.to_owned(),
        data: related_model
            .as_table()
            .alias(right_table_alias)
            .on(ConditionTree::And(on_conditions)),
    }
}
