use crate::{filter::FilterBuilder, model_extensions::*, Context};
use quaint::prelude::*;
use query_structure::*;

#[derive(Debug, Clone)]
pub(crate) struct AliasedJoin {
    // Actual join data to be passed to quaint
    pub(crate) data: Join<'static>,
    // Alias used for the join. eg: LEFT JOIN ... AS <alias>
    pub(crate) alias: String,
}

#[derive(Debug, Clone)]
pub(crate) enum AggregationType {
    Count,
}

pub(crate) fn compute_aggr_join(
    rf: &RelationFieldRef,
    aggregation: AggregationType,
    filter: Option<Filter>,
    aggregator_alias: &str,
    join_alias: &str,
    previous_join: Option<&str>,
    ctx: &Context<'_>,
) -> AliasedJoin {
    let join_alias = format!("{}_{}", join_alias, &rf.related_model().name());

    if rf.relation().is_many_to_many() {
        compute_aggr_join_m2m(
            rf,
            aggregation,
            filter,
            aggregator_alias,
            join_alias.as_str(),
            previous_join,
            ctx,
        )
    } else {
        compute_aggr_join_one2m(
            rf,
            aggregation,
            filter,
            aggregator_alias,
            join_alias.as_str(),
            previous_join,
            ctx,
        )
    }
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
    previous_join: Option<&str>,
    ctx: &Context<'_>,
) -> AliasedJoin {
    let (left_fields, right_fields) = if rf.is_inlined_on_enclosing_model() {
        (rf.scalar_fields(), rf.referenced_fields())
    } else {
        (
            rf.related_field().referenced_fields(),
            rf.related_field().scalar_fields(),
        )
    };
    let select_columns = right_fields.iter().map(|f| f.as_column(ctx));
    let (conditions, joins) = filter
        .map(|f| FilterBuilder::with_top_level_joins().visit_filter(f, ctx))
        .unwrap_or((ConditionTree::NoCondition, None));

    // + SELECT Child.<fk> FROM Child WHERE <FILTER>
    let query = Select::from_table(rf.related_model().as_table(ctx))
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
    let query = right_fields.iter().fold(query, |acc, f| acc.group_by(f.as_column(ctx)));

    let query = if let Some(joins) = joins {
        joins.into_iter().fold(query, |acc, join| acc.join(join.data))
    } else {
        query
    };

    let pairs = left_fields.into_iter().zip(right_fields);
    let on_conditions: Vec<Expression> = pairs
        .map(|(a, b)| {
            let col_a = match previous_join {
                Some(prev_join) => Column::from((prev_join.to_owned(), a.db_name().to_owned())),
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
    previous_join: Option<&str>,
    ctx: &Context<'_>,
) -> AliasedJoin {
    // m2m join table (_ParentToChild)
    let m2m_table = rf.as_table(ctx);
    // Child colums on the m2m join table (_ParentToChild.ChildId)
    let m2m_child_column = rf.related_field().m2m_column(ctx);
    // Child table
    let child_model = rf.related_model();
    // Child primary identifiers
    let child_ids: ModelProjection = rf.related_model().primary_identifier().into();
    // Parent primary identifiers
    let parent_ids: ModelProjection = rf.model().primary_identifier().into();
    // Rendered filters
    let (conditions, joins) = filter
        .map(|f| FilterBuilder::with_top_level_joins().visit_filter(f, ctx))
        .unwrap_or((ConditionTree::NoCondition, None));

    // + SELECT _ParentToChild.ChildId FROM Child WHERE <FILTER>
    let query = Select::from_table(child_model.as_table(ctx))
        .columns([m2m_child_column.clone()])
        .so_that(conditions);

    let query = if let Some(joins) = joins {
        joins.into_iter().fold(query, |acc, join| acc.join(join.data))
    } else {
        query
    };

    let aggr_expr = match aggregation {
        AggregationType::Count => count(m2m_child_column.clone()),
    };

    // SELECT _ParentToChild.ChildId,
    // + COUNT(_ParentToChild.ChildId) AS <AGGREGATOR_ALIAS>
    // FROM Child WHERE <FILTER>
    let query = query.value(aggr_expr.alias(aggregator_alias.to_owned()));

    let left_join_conditions: Vec<Expression> = child_ids
        .as_columns(ctx)
        .map(|c| c.equals(rf.m2m_column(ctx)).into())
        .collect();

    // SELECT _ParentToChild.ChildId, COUNT(_ParentToChild.ChildId) AS <AGGREGATOR_ALIAS> FROM Child WHERE <FILTER>
    // + LEFT JOIN _ParentToChild ON (Child.id = _ParenTtoChild.ChildId)
    let query = query.left_join(m2m_table.on(ConditionTree::And(left_join_conditions)));

    // SELECT _ParentToChild.ChildId, COUNT(_ParentToChild.ChildId) AS <AGGREGATOR_ALIAS> FROM Child WHERE <FILTER>
    // LEFT JOIN _ParentToChild ON (Child.id = _ParentToChild.ChildId)
    // + GROUP BY _ParentToChild.ChildId
    let query = query.group_by(rf.related_field().m2m_column(ctx));

    let (left_fields, right_fields) = (parent_ids.scalar_fields(), [m2m_child_column]);
    let pairs = left_fields.zip(right_fields);
    let on_conditions: Vec<Expression> = pairs
        .map(|(a, b)| {
            let col_a = match previous_join {
                Some(prev_join) => Column::from((prev_join.to_owned(), a.db_name().to_owned())),
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
        alias: join_alias.to_owned(),
        data: Join::Left(join),
    }
}

pub(crate) fn compute_one2m_join(
    field: &RelationFieldRef,
    alias: &str,
    parent_alias: Option<&str>,
    ctx: &Context<'_>,
) -> AliasedJoin {
    let join_columns: Vec<Column> = field
        .join_columns(ctx)
        .map(|c| c.opt_table(parent_alias.map(ToOwned::to_owned)))
        .collect();

    let related_table = field.related_model().as_table(ctx);
    let related_join_columns: Vec<_> = ModelProjection::from(field.related_field().linking_fields())
        .as_columns(ctx)
        .map(|col| col.table(alias.to_owned()))
        .collect();

    let join = related_table
        .alias(alias.to_owned())
        .on(Row::from(related_join_columns).equals(Row::from(join_columns)));

    AliasedJoin {
        alias: alias.to_owned(),
        data: Join::Left(join),
    }
}
