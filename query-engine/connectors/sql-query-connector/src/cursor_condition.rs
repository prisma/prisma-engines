use connector_interface::QueryArguments;
use prisma_models::*;
use quaint::ast::*;

pub fn build(query_arguments: &QueryArguments, model: ModelRef) -> ConditionTree<'static> {
    match (query_arguments.cursor.as_ref(), query_arguments.order_by.as_ref()) {
        (None, _) => ConditionTree::NoCondition,
        (Some(cursor), order_by) => {
            // If there's a sort order defined for the cursor, take that one, else implicitly order by ID.
            let (comparison_fields, sort_order) = match order_by {
                Some(x) => (vec![x.field.clone()], x.sort_order),
                None => (
                    model.primary_identifier().scalar_fields().collect(),
                    SortOrder::Ascending,
                ),
            };

            let columns: Vec<_> = comparison_fields.as_columns().collect();
            let order_row = Row::from(columns.clone());
            let fields: Vec<_> = cursor.fields().collect();
            let values: Vec<_> = cursor.values().collect();

            let cursor_columns: Vec<_> = fields.as_slice().as_columns().collect();
            let cursor_row = Row::from(cursor_columns);

            let where_condition = cursor_row.clone().equals(values.clone());

            let select_query = Select::from_table(model.as_table())
                .columns(columns.clone())
                .so_that(where_condition);

            // A negative `take` value signifies that values should be taken before the cursor, requiring a different ordering.
            let compare = match (query_arguments.take, sort_order) {
                (Some(t), SortOrder::Ascending) if t < 0 => order_row
                    .clone()
                    .equals(select_query.clone())
                    .and(cursor_row.clone().less_than_or_equals(values))
                    .or(order_row.less_than_or_equals(select_query)),

                (Some(t), SortOrder::Descending) if t < 0 => order_row
                    .clone()
                    .equals(select_query.clone())
                    .and(cursor_row.clone().less_than_or_equals(values))
                    .or(order_row.greater_than_or_equals(select_query)),

                (_, SortOrder::Ascending) => order_row
                    .clone()
                    .equals(select_query.clone())
                    .and(cursor_row.clone().greater_than_or_equals(values))
                    .or(order_row.greater_than_or_equals(select_query)),

                (_, SortOrder::Descending) => order_row
                    .clone()
                    .equals(select_query.clone())
                    .and(cursor_row.clone().greater_than_or_equals(values))
                    .or(order_row.less_than_or_equals(select_query)),
            };

            ConditionTree::single(compare)
        }
    }
}
