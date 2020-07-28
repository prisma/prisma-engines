use crate::query_arguments_ext::QueryArgumentsExt;
use connector_interface::QueryArguments;
use prisma_models::*;
use quaint::ast::*;

/// Builds all expressions for an `ORDER BY` clause based on the query arguments.
pub fn build(query_arguments: &QueryArguments) -> Vec<OrderDefinition<'static>> {
    let needs_reversed_order = query_arguments.needs_reversed_order();

    query_arguments.order_by.iter().fold(vec![], |mut acc, next_order_by| {
        match (next_order_by.sort_order, needs_reversed_order) {
            (SortOrder::Ascending, true) => acc.push(next_order_by.field.as_column().descend()),
            (SortOrder::Descending, true) => acc.push(next_order_by.field.as_column().ascend()),
            (SortOrder::Ascending, false) => acc.push(next_order_by.field.as_column().ascend()),
            (SortOrder::Descending, false) => acc.push(next_order_by.field.as_column().descend()),
        }

        acc
    })
}
