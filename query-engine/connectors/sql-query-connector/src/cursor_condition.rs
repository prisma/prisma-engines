use connector_interface::QueryArguments;
use prisma_models::*;
use quaint::ast::*;

static ORDER_TABLE_ALIAS: &'static str = "order_cmp";

pub fn build(query_arguments: &QueryArguments, model: &ModelRef) -> (Option<Table<'static>>, ConditionTree<'static>) {
    match query_arguments.cursor {
        None => (None, ConditionTree::NoCondition),
        Some(ref cursor) => {
            let cursor_fields: Vec<_> = cursor.fields().collect();
            let cursor_values: Vec<_> = cursor.values().collect();
            let cursor_columns: Vec<_> = cursor_fields.as_slice().as_columns().collect();
            let cursor_row = Row::from(cursor_columns);

            // Invariant: Cursors are unique. This means we can create a subquery to find at most one row
            // that contains all the values required for the odering row comparison (order_subquery).
            let cursor_condition = cursor_row.clone().equals(cursor_values.clone());

            // Orderings for this query. Influences which fields we need to fetch for comparing order fields.
            let order_definitions = order_definitions(query_arguments, model);

            // Subquery to find the value of the order field(s) that we need for comparison.
            let order_subquery = order_definitions
                .iter()
                .fold(Select::from_table(model.as_table()), |select, (field, order)| {
                    select.column(field.as_column())
                })
                .so_that(cursor_condition);

            let subquery_table = Table::from(order_subquery).alias(ORDER_TABLE_ALIAS);

            // SELECT
            //     "TestModel".id
            // FROM
            //     "TestModel",
            //     (SELECT fieldA, fieldB FROM "TestModel" WHERE "TestModel".id = 4) as order_cmp -- Find order row values
            // WHERE
            // 	   TestModel.fieldA <= order_cmp.fieldA AND TestModel.fieldB >= order_cmp.fieldB
            // ORDER BY
            //     "TestModel"."fieldA" DESC,
            //     "TestModel"."fieldB" ASC
            // LIMIT
            //    2 OFFSET 1;

            let conditions: Vec<_> = order_definitions
                .into_iter()
                .map(|(field, order)| {
                    let order_column = field.as_column();

                    // A negative `take` value signifies that values should be taken before the cursor,
                    // requiring the correct comarison operator to be used to fit the reversed order.
                    match (query_arguments.take, order) {
                        // If it's ASC but we want to take from the back, the ORDER BY will be DESC, meaning that comparisons done need to be lte.
                        (Some(t), SortOrder::Ascending) if t < 0 => order_column
                            .less_than_or_equals(Column::from((ORDER_TABLE_ALIAS, field.db_name().to_owned()))),

                        // If it's DESC but we want to take from the back, the ORDER BY will be ASC, meaning that comparisons done need to be gte.
                        (Some(t), SortOrder::Descending) if t < 0 => order_column
                            .greater_than_or_equals(Column::from((ORDER_TABLE_ALIAS, field.db_name().to_owned()))),

                        // Sorting is unchanged.
                        (_, SortOrder::Ascending) => order_column
                            .greater_than_or_equals(Column::from((ORDER_TABLE_ALIAS, field.db_name().to_owned()))),

                        // Sorting is unchanged.
                        (_, SortOrder::Descending) => order_column
                            .less_than_or_equals(Column::from((ORDER_TABLE_ALIAS, field.db_name().to_owned()))),
                    }
                    .into()
                })
                .collect();

            // let conditions: Vec<_> = order_definitions
            //     .into_iter()
            //     .map(|(field, order)| {
            //         let order_column = field.as_column();

            //         // Subquery to find the value of the order field(s) that we need for comparison.
            //         let order_value_subselect = Select::from_table(table.clone())
            //             .column(order_column.clone())
            //             .so_that(cursor_condition.clone());

            //         // A negative `take` value signifies that values should be taken before the cursor, requiring a sort reversal.
            //         match (query_arguments.take, order) {
            //             (Some(t), SortOrder::Ascending) if t < 0 => order_column
            //                 .clone()
            //                 .equals(order_value_subselect.clone())
            //                 .and(cursor_row.clone().less_than_or_equals(cursor_values.clone()))
            //                 .or(order_column.less_than(order_value_subselect)),

            //             (Some(t), SortOrder::Descending) if t < 0 => order_column
            //                 .clone()
            //                 .equals(order_value_subselect.clone())
            //                 .and(cursor_row.clone().less_than_or_equals(cursor_values.clone()))
            //                 .or(order_column.greater_than(order_value_subselect)),

            //             (_, SortOrder::Ascending) => order_column
            //                 .clone()
            //                 .equals(order_value_subselect.clone())
            //                 .and(cursor_row.clone().greater_than_or_equals(cursor_values.clone()))
            //                 .or(order_column.greater_than(order_value_subselect)),

            //             (_, SortOrder::Descending) => order_column
            //                 .clone()
            //                 .equals(order_value_subselect.clone())
            //                 .and(cursor_row.clone().greater_than_or_equals(cursor_values.clone()))
            //                 .or(order_column.less_than(order_value_subselect)),
            //         }
            //         .into()
            //     })
            //     .collect();

            // ConditionTree::And(conditions)

            (Some(subquery_table), ConditionTree::And(conditions))
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
