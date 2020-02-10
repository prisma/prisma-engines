use connector_interface::QueryArguments;
use prisma_models::*;
use quaint::ast::*;

#[derive(Clone, Copy)]
enum CursorType {
    Before,
    After,
}

pub fn build(query_arguments: &QueryArguments, model: ModelRef) -> ConditionTree<'static> {
    match (
        query_arguments.before.as_ref(),
        query_arguments.after.as_ref(),
        query_arguments.order_by.as_ref(),
    ) {
        (None, None, _) => ConditionTree::NoCondition,
        (before, after, order_by) => {
            let id_field = model.fields().find_singular_id().unwrap().upgrade().unwrap();
            let (comparison_field, sort_order) = match order_by {
                Some(x) => (&x.field, x.sort_order),
                None => (&id_field, SortOrder::Ascending),
            };

            let cursor_for = |cursor_type: CursorType, pairs: &[(ScalarFieldRef, PrismaValue)]| {
                let (fields, values): (Vec<_>, Vec<_>) = pairs.iter().cloned().unzip();
                let columns: Vec<_> = vec![comparison_field.as_column()];
                let order_row = Row::from(columns.clone());

                let cursor_columns: Vec<_> = fields.into_iter().map(|sf| sf.as_column()).collect();
                let cursor_row = Row::from(cursor_columns);

                let where_condition = cursor_row.clone().equals(values.clone());

                let select_query = Select::from_table(model.as_table())
                    .columns(columns.clone())
                    .so_that(where_condition);

                let compare = match (cursor_type, sort_order) {
                    (CursorType::Before, SortOrder::Ascending) => order_row
                        .clone()
                        .equals(select_query.clone())
                        .and(cursor_row.clone().less_than(values))
                        .or(order_row.less_than(select_query)),

                    (CursorType::Before, SortOrder::Descending) => order_row
                        .clone()
                        .equals(select_query.clone())
                        .and(cursor_row.clone().less_than(values))
                        .or(order_row.greater_than(select_query)),

                    (CursorType::After, SortOrder::Ascending) => order_row
                        .clone()
                        .equals(select_query.clone())
                        .and(cursor_row.clone().greater_than(values))
                        .or(order_row.greater_than(select_query)),

                    (CursorType::After, SortOrder::Descending) => order_row
                        .clone()
                        .equals(select_query.clone())
                        .and(cursor_row.clone().greater_than(values))
                        .or(order_row.less_than(select_query)),
                };

                ConditionTree::single(compare)
            };

            let after_cursor = after
                .map(|pairs| cursor_for(CursorType::After, pairs))
                .unwrap_or(ConditionTree::NoCondition);

            let before_cursor = before
                .map(|pairs| cursor_for(CursorType::Before, pairs))
                .unwrap_or(ConditionTree::NoCondition);

            ConditionTree::and(after_cursor, before_cursor)
        }
    }
}
