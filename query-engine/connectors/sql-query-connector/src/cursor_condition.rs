use connector_interface::QueryArguments;
use prisma_models::*;
use quaint::ast::*;
use std::sync::Arc;

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
            let field = match order_by {
                Some(order) => ModelIdentifier::from(Arc::clone(&order.field)),
                None => model.identifier(),
            };

            let sort_order: SortOrder = order_by.map(|order| order.sort_order).unwrap_or(SortOrder::Ascending);

            let cursor_for = |cursor_type: CursorType, id: &RecordIdentifier| {
                let id_values: Row = {
                    let id_values: Vec<PrismaValue> = id.values().collect();
                    id_values.into()
                };

                let model_id = model.identifier();
                let where_condition = model_id.as_row().equals(id_values.clone());

                let select_query = Select::from_table(model.as_table())
                    .columns(field.as_columns())
                    .so_that(ConditionTree::single(where_condition));

                let compare = match (cursor_type, sort_order) {
                    (CursorType::Before, SortOrder::Ascending) => field
                        .as_row()
                        .equals(select_query.clone())
                        .and(model_id.as_row().less_than(id_values.clone()))
                        .or(field.as_row().less_than(select_query)),
                    (CursorType::Before, SortOrder::Descending) => field
                        .as_row()
                        .equals(select_query.clone())
                        .and(model_id.as_row().less_than(id_values.clone()))
                        .or(field.as_row().greater_than(select_query)),
                    (CursorType::After, SortOrder::Ascending) => field
                        .as_row()
                        .equals(select_query.clone())
                        .and(model_id.as_row().greater_than(id_values.clone()))
                        .or(field.as_row().greater_than(select_query)),
                    (CursorType::After, SortOrder::Descending) => field
                        .as_row()
                        .equals(select_query.clone())
                        .and(model_id.as_row().greater_than(id_values.clone()))
                        .or(field.as_row().less_than(select_query)),
                };

                ConditionTree::single(compare)
            };

            let after_cursor = after
                .map(|id| {
                    cursor_for(CursorType::After, id)
                })
                .unwrap_or(ConditionTree::NoCondition);

            let before_cursor = before
                .map(|id| {
                    cursor_for(CursorType::Before, id)
                })
                .unwrap_or(ConditionTree::NoCondition);

            ConditionTree::and(after_cursor, before_cursor)
        }
    }
}
