use crate::query_arguments_ext::QueryArgumentsExt;
use connector_interface::QueryArguments;
use prisma_models::*;
use quaint::ast::*;

static ORDER_JOIN_PREFIX: &str = "orderby_";

/// Builds all expressions for an `ORDER BY` clause based on the query arguments.
pub fn build(
    query_arguments: &QueryArguments,
    base_model: &ModelRef, // The model the ordering will start from
) -> (Vec<OrderDefinition<'static>>, Vec<JoinData<'static>>) {
    let needs_reversed_order = query_arguments.needs_reversed_order();

    let mut order_definitions = vec![];
    let mut joins = vec![];

    // The index is used to differentiate potentially separate relations to the same model.
    for (index, order_by) in query_arguments.order_by.iter().enumerate() {
        let (mut computed_joins, order_by_column) = compute_joins(order_by, base_model, index);

        joins.append(&mut computed_joins);

        match (order_by.sort_order, needs_reversed_order) {
            (SortOrder::Ascending, true) => order_definitions.push(order_by_column.descend()),
            (SortOrder::Descending, true) => order_definitions.push(order_by_column.ascend()),
            (SortOrder::Ascending, false) => order_definitions.push(order_by_column.ascend()),
            (SortOrder::Descending, false) => order_definitions.push(order_by_column.descend()),
        }
    }

    (order_definitions, joins)
}

pub fn compute_joins(
    order_by: &OrderBy,
    base_model: &ModelRef,
    order_by_index: usize,
) -> (Vec<JoinData<'static>>, Column<'static>) {
    let join_prefix = format!("{}{}", ORDER_JOIN_PREFIX, order_by_index);
    let mut joins = vec![];
    let mut last_join_alias: Option<String> = None;

    for rf in order_by.path.iter() {
        let (join_alias, join) = compute_join(base_model, rf, join_prefix.as_str());

        last_join_alias = Some(join_alias);
        joins.push(join);
    }

    // This is the final column identifier to be used for the scalar field to order by.
    // - If it's on the base model with no hops, it's for example `modelTable.field`.
    // - If it is with several hops, it's the alias used for the last join, e.g.
    //   `{join_alias}.field`
    let order_by_column = build_order_by_column(order_by, last_join_alias);

    (joins, order_by_column)
}

fn build_order_by_column(order_by: &OrderBy, join_alias: Option<String>) -> Column<'static> {
    if join_alias.is_some() {
        Column::from((join_alias.unwrap().to_owned(), order_by.field.db_name().to_owned()))
    } else {
        order_by.field.as_column()
    }
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
