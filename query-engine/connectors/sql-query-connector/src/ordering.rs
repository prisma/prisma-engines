use crate::query_arguments_ext::QueryArgumentsExt;
use connector_interface::QueryArguments;
use prisma_models::*;
use quaint::ast::*;

static ORDER_JOIN_PREFIX: &str = "orderby_";
static ORDER_AGGR_FIELD_NAME: &str = "orderby_aggregator";

#[derive(Debug, Clone)]
pub struct OrderingJoin {
    // Actual join data to be passed to quaint
    pub(crate) data: JoinData<'static>,
    // Alias used for the join. eg: LEFT JOIN ... AS <alias>
    pub(crate) alias: String,
}
#[derive(Debug, Clone)]
pub struct OrderingJoins {
    // Joins necessary to perform the order by
    pub(crate) joins: Vec<OrderingJoin>,
    // Final column identifier to be used for the scalar field to order by
    pub(crate) order_column: Column<'static>,
}

/// Builds all expressions for an `ORDER BY` clause based on the query arguments.
pub fn build(
    query_arguments: &QueryArguments,
    base_model: &ModelRef, // The model the ordering will start from
) -> (Vec<OrderDefinition<'static>>, Vec<OrderingJoins>) {
    let needs_reversed_order = query_arguments.needs_reversed_order();

    let mut order_definitions = vec![];
    let mut ordering_joins = vec![];

    // The index is used to differentiate potentially separate relations to the same model.
    for (index, order_by) in query_arguments.order_by.iter().enumerate() {
        let (computed_joins, order_column) = compute_joins(order_by, index, base_model);

        ordering_joins.push(OrderingJoins {
            joins: computed_joins,
            order_column: order_column.clone(),
        });

        match (order_by.sort_order, needs_reversed_order) {
            (SortOrder::Ascending, true) => order_definitions.push(order_column.descend()),
            (SortOrder::Descending, true) => order_definitions.push(order_column.ascend()),
            (SortOrder::Ascending, false) => order_definitions.push(order_column.ascend()),
            (SortOrder::Descending, false) => order_definitions.push(order_column.descend()),
        }
    }

    (order_definitions, ordering_joins)
}

pub fn compute_joins(
    order_by: &OrderBy,
    order_by_index: usize,
    base_model: &ModelRef,
) -> (Vec<OrderingJoin>, Column<'static>) {
    let join_prefix = format!("{}{}", ORDER_JOIN_PREFIX, order_by_index);
    let mut joins = vec![];
    let mut order_by_column_alias: Option<String> = None;
    let last_path = order_by.path.last();

    for rf in order_by.path.iter() {
        if order_by.sort_aggregation.is_some() && Some(rf) == last_path {
            let (aggr_alias, ordering_join) = compute_aggr_join(order_by, rf, join_prefix.as_str(), joins.last());

            order_by_column_alias = Some(aggr_alias);
            joins.push(ordering_join);
        } else {
            let ordering_join = compute_join(base_model, rf, join_prefix.as_str());

            joins.push(ordering_join);
        }
    }

    // This is the final column identifier to be used for the scalar field to order by.
    // - If it's on the base model with no hops, it's for example `modelTable.field`.
    // - If it is with several hops, it's the alias used for the last join, e.g.
    //   `{join_alias}.field`
    // - If it's with an order by aggregation, it's the alias used for the join + alias used for the aggregator. eg:
    //   `{join_alias}.{aggr_column_alias}`
    let order_by_column = if let Some(join) = joins.last() {
        Column::from((
            join.alias.to_owned(),
            order_by_column_alias.unwrap_or_else(|| order_by.field.db_name().to_owned()),
        ))
    } else {
        order_by.field.as_column()
    };

    (joins, order_by_column)
}

fn compute_join(base_model: &ModelRef, rf: &RelationFieldRef, join_prefix: &str) -> OrderingJoin {
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

    let on_conditions = pairs
        .map(|(a, b)| {
            let a_col = if let Some(alias) = left_table_alias.clone() {
                Column::from((alias, a.db_name().to_owned()))
            } else {
                a.as_column()
            };

            let b_col = Column::from((right_table_alias.clone(), b.db_name().to_owned()));

            a_col.equals(b_col)
        })
        .collect::<Vec<_>>();

    OrderingJoin {
        alias: right_table_alias.to_owned(),
        data: related_model
            .as_table()
            .alias(right_table_alias)
            .on(ConditionTree::single(on_conditions)),
    }
}

fn compute_aggr_join(
    order_by: &OrderBy,
    rf: &RelationFieldRef,
    join_alias: &str,
    previous_join: Option<&OrderingJoin>,
) -> (String, OrderingJoin) {
    let join_alias = format!("{}_{}", join_alias, &rf.related_model().name);

    if rf.relation().is_many_to_many() {
        compute_aggr_join_m2m(order_by, rf, join_alias.as_str(), previous_join)
    } else {
        compute_aggr_join_one2m(order_by, rf, join_alias.as_str(), previous_join)
    }
}

fn compute_aggr_join_one2m(
    order_by: &OrderBy,
    rf: &RelationFieldRef,
    join_alias: &str,
    previous_join: Option<&OrderingJoin>,
) -> (String, OrderingJoin) {
    let (left_fields, right_fields) = if rf.is_inlined_on_enclosing_model() {
        (rf.scalar_fields(), rf.referenced_fields())
    } else {
        (
            rf.related_field().referenced_fields(),
            rf.related_field().scalar_fields(),
        )
    };
    let select_columns = right_fields.iter().map(|f| f.as_column());

    // + SELECT A.fk FROM A
    let query = Select::from_table(rf.related_model().as_table()).columns(select_columns);
    let aggr_expr = match order_by
        .sort_aggregation
        .expect("This function should be guaranteed to be passed a sort_aggregation")
    {
        SortAggregation::Count { _all } => count(asterisk()),
    };

    // SELECT A.fk,
    // + COUNT(*) AS <ORDER_AGGR_FIELD_NAME>
    // FROM A
    let query = query.value(aggr_expr.alias(ORDER_AGGR_FIELD_NAME.to_owned()));

    // SELECT A.<fk>, COUNT(*) AS <ORDER_AGGR_FIELD_NAME> FROM A
    // + GROUP BY A.<fk>
    let query = right_fields.iter().fold(query, |acc, f| acc.group_by(f.as_column()));

    let pairs = left_fields.into_iter().zip(right_fields.into_iter());
    let on_conditions = pairs
        .map(|(a, b)| {
            let col_a = match previous_join {
                Some(prev_join) => Column::from((prev_join.alias.to_owned(), a.db_name().to_owned())),
                None => a.as_column(),
            };
            let col_b = Column::from((join_alias.to_owned(), b.db_name().to_owned()));

            col_a.equals(col_b)
        })
        .collect::<Vec<_>>();

    // + LEFT JOIN (
    //     SELECT A.<fk>, COUNT(*) AS <ORDER_AGGR_FIELD_NAME> FROM A
    //     GROUP BY A.<fk>
    // + ) AS <ORDER_JOIN_PREFIX> ON (<A | previous_join_alias>.<fk> = <ORDER_JOIN_PREFIX>.<fk>)
    let join = Table::from(query)
        .alias(join_alias.to_owned())
        .on(ConditionTree::single(on_conditions));

    (
        ORDER_AGGR_FIELD_NAME.to_owned(),
        OrderingJoin {
            data: join,
            alias: join_alias.to_owned(),
        },
    )
}

fn compute_aggr_join_m2m(
    order_by: &OrderBy,
    rf: &RelationFieldRef,
    join_alias: &str,
    previous_join: Option<&OrderingJoin>,
) -> (String, OrderingJoin) {
    let relation_table = rf.as_table();
    let a_ids = rf.model().primary_identifier();
    let b_ids = rf.related_model().primary_identifier();

    // + SELECT A.id FROM _AtoB
    let query = Select::from_table(relation_table).columns(a_ids.as_columns());

    let aggr_expr = match order_by
        .sort_aggregation
        .expect("This function should be guaranteed to be passed a sort_aggregation")
    {
        SortAggregation::Count { _all } => count(asterisk()),
    };
    // SELECT A.id,
    // + COUNT(*) AS <ORDER_AGGR_FIELD_NAME>
    // FROM _AtoB
    let query = query.value(aggr_expr.alias(ORDER_AGGR_FIELD_NAME.to_owned()));

    let conditions_a: Vec<_> = a_ids
        .as_columns()
        .map(|c| c.equals(rf.related_field().m2m_columns()))
        .collect();
    let conditions_b: Vec<_> = b_ids.as_columns().map(|c| c.equals(rf.m2m_columns())).collect();

    // SELECT A.id, COUNT(*) AS <ORDER_AGGR_FIELD_NAME> FROM _AtoB
    // + INNER JOIN A ON A.id = _AtoB.A
    // + INNER JOIN B ON B.id = _AtoB.B
    let query = query
        .inner_join(rf.model().as_table().on(ConditionTree::single(conditions_a)))
        .inner_join(rf.related_model().as_table().on(ConditionTree::single(conditions_b)));

    // SELECT A.id, COUNT(*) AS <ORDER_AGGR_FIELD_NAME> FROM _AtoB
    // INNER JOIN A ON A.id = _AtoB.A
    // INNER JOIN B ON B.id = _AtoB.B
    // + GROUP BY A.id
    let query = a_ids.as_columns().fold(query, |acc, f| acc.group_by(f.clone()));

    let (left_fields, right_fields) = (
        a_ids.scalar_fields().collect::<Vec<_>>(),
        b_ids.scalar_fields().collect::<Vec<_>>(),
    );
    let pairs = left_fields.into_iter().zip(right_fields.into_iter());
    let on_conditions = pairs
        .map(|(a, b)| {
            let col_a = match previous_join {
                Some(prev_join) => Column::from((prev_join.alias.to_owned(), a.db_name().to_owned())),
                None => a.as_column(),
            };
            let col_b = Column::from((join_alias.to_owned(), b.db_name().to_owned()));

            col_a.equals(col_b)
        })
        .collect::<Vec<_>>();

    // + LEFT JOIN (
    //     SELECT A.id, COUNT(*) AS <ORDER_AGGR_FIELD_NAME> FROM _AtoB
    //       INNER JOIN A ON (A.id = _AtoB.A)
    //       INNER JOIN B ON (B.id = _AtoB.B)
    //     GROUP BY A.id
    // + ) AS <ORDER_JOIN_PREFIX> ON (<A | previous_join_alias >.id = <ORDER_JOIN_PREFIX>.id)
    let join = Table::from(query)
        .alias(join_alias.to_owned())
        .on(ConditionTree::single(on_conditions));

    (
        ORDER_AGGR_FIELD_NAME.to_owned(),
        OrderingJoin {
            alias: join_alias.to_owned(),
            data: join,
        },
    )
}
