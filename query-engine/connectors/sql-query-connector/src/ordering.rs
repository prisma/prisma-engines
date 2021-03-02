use crate::query_arguments_ext::QueryArgumentsExt;
use connector_interface::QueryArguments;
use prisma_models::*;
use quaint::ast::*;

static ORDER_JOIN_PREFIX: &str = "orderby_";

#[derive(Debug, Clone)]
pub struct AliasedJoin {
    // Actual join data to be passed to quaint
    pub(crate) data: JoinData<'static>,
    // Alias used for the join. eg: LEFT JOIN ... AS <alias>
    pub(crate) alias: String,
}
#[derive(Debug, Clone)]
pub struct OrderingJoins {
    // Joins necessary to perform the order by
    pub(crate) joins: Vec<AliasedJoin>,
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
) -> (Vec<AliasedJoin>, Column<'static>) {
    let join_prefix = format!("{}{}", ORDER_JOIN_PREFIX, order_by_index);
    let mut joins = vec![];

    for rf in order_by.path.iter() {
        let (join_alias, join_data) = compute_join(base_model, rf, join_prefix.as_str());

        joins.push(AliasedJoin {
            data: join_data,
            alias: join_alias,
        });
    }

    // This is the final column identifier to be used for the scalar field to order by.
    // - If it's on the base model with no hops, it's for example `modelTable.field`.
    // - If it is with several hops, it's the alias used for the last join, e.g.
    //   `{join_alias}.field`
    let order_by_column = if let Some(join) = joins.last() {
        Column::from((join.alias.to_owned(), order_by.field.db_name().to_owned()))
    } else {
        order_by.field.as_column()
    };

    (joins, order_by_column)
}

fn compute_join(base_model: &ModelRef, rf: &RelationFieldRef, join_prefix: &str) -> (String, JoinData<'static>) {
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

    (
        right_table_alias.to_owned(),
        related_model
            .as_table()
            .alias(right_table_alias)
            .on(ConditionTree::single(on_conditions)),
    )
}
