use connector_interface::QueryArguments;
use prisma_models::*;
use quaint::ast::*;

pub fn build(query_arguments: &QueryArguments, model: ModelRef) -> ConditionTree<'static> {
    match query_arguments.cursor {
        None => ConditionTree::NoCondition,
        Some(ref cursor) => {
            let cursor_fields: Vec<_> = cursor.fields().collect();
            let cursor_values: Vec<_> = cursor.values().collect();
            let cursor_columns: Vec<_> = cursor_fields.as_slice().as_columns().collect();
            let cursor_row = Row::from(cursor_columns);

            // Cursor columns need to equal the given values.
            let cursor_condition = cursor_row.clone().equals(cursor_values.clone());

            let order_definitions = order_definitions(query_arguments, &model);
            let table = model.as_table();
            let conditions: Vec<_> = order_definitions
                .into_iter()
                .map(|(field, order)| {
                    let order_column = field.as_column();

                    // Subquery to find the value of the order field(s) that we need for comparison.
                    let order_value_subselect = Select::from_table(table.clone())
                        .column(order_column.clone())
                        .so_that(cursor_condition.clone());

                    // A negative `take` value signifies that values should be taken before the cursor, requiring a sort reversal.
                    match (query_arguments.take, order) {
                        (Some(t), SortOrder::Ascending) if t < 0 => order_column
                            .clone()
                            .equals(order_value_subselect.clone())
                            .and(cursor_row.clone().less_than_or_equals(cursor_values.clone()))
                            .or(order_column.less_than(order_value_subselect)),

                        (Some(t), SortOrder::Descending) if t < 0 => order_column
                            .clone()
                            .equals(order_value_subselect.clone())
                            .and(cursor_row.clone().less_than_or_equals(cursor_values.clone()))
                            .or(order_column.greater_than(order_value_subselect)),

                        (_, SortOrder::Ascending) => order_column
                            .clone()
                            .equals(order_value_subselect.clone())
                            .and(cursor_row.clone().greater_than_or_equals(cursor_values.clone()))
                            .or(order_column.greater_than(order_value_subselect)),

                        (_, SortOrder::Descending) => order_column
                            .clone()
                            .equals(order_value_subselect.clone())
                            .and(cursor_row.clone().greater_than_or_equals(cursor_values.clone()))
                            .or(order_column.less_than(order_value_subselect)),
                    }
                    .into()
                })
                .collect();

            ConditionTree::And(conditions)
        }
    }
}

fn order_definitions(query_arguments: &QueryArguments, model: &ModelRef) -> Vec<(ScalarFieldRef, SortOrder)> {
    let defined_ordering: Vec<_> = query_arguments
        .order_by
        .iter()
        .map(|o| (o.field.clone(), o.sort_order))
        .collect();

    if defined_ordering.is_empty() {
        model
            .primary_identifier()
            .scalar_fields()
            .map(|f| (f, SortOrder::Ascending))
            .collect()
    } else {
        defined_ordering
    }
}
