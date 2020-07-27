use connector_interface::QueryArguments;
use prisma_models::*;
use quaint::ast::*;

/// Builds
pub fn build(query_arguments: &QueryArguments, model: &ModelRef) -> Vec<OrderDefinition<'static>> {
    let needs_reversed_order = needs_reversed_order(query_arguments);

    for order in query_arguments.order_by.iter() {
        match (order_by.sort_order, needs_reversed_order) {
            (SortOrder::Ascending, true) => {
                Self::merge_columns(first.descend(), identifier.into_iter().map(|c| c.descend()), size_hint)
            }
            (SortOrder::Descending, true) => {
                Self::merge_columns(first.ascend(), identifier.into_iter().map(|c| c.descend()), size_hint)
            }
            (SortOrder::Ascending, false) => {
                Self::merge_columns(first.ascend(), identifier.into_iter().map(|c| c.ascend()), size_hint)
            }
            (SortOrder::Descending, false) => {
                Self::merge_columns(first.descend(), identifier.into_iter().map(|c| c.ascend()), size_hint)
            }
        }
    }

    todo!()
}

// -------------

// impl QueryArguments {
// pub fn ordering_directions(&self) -> OrderDirections {
//     OrderDirections {
//         needs_implicit_id_ordering: self.needs_implicit_ordering(),
//         primary_order_by: self.order_by.clone(),
//     }
// }

/// If we need to take rows before a cursor position, then we need to reverse the order in SQL.
fn needs_reversed_order(args: &QueryArguments) -> bool {
    args.take.map(|t| t < 0).unwrap_or(false)
}

type OrderVec<'a> = Vec<(Expression<'a>, Option<Order>)>;

struct OrderDirections {
    pub needs_implicit_id_ordering: bool,
    pub primary_order_by: Option<OrderBy>,
    pub needs_to_be_reverse_order: bool,
}

struct Ordering;

/// Tooling for generating orderings for different query types.
impl Ordering {
    pub fn for_model(model: &ModelRef, order_directive: OrderDirections) -> OrderVec<'static> {
        Self::by_fields(
            order_directive
                .primary_order_by
                .as_ref()
                .map(|oby| oby.field.as_column()),
            model.primary_identifier().as_columns().collect(),
            order_directive,
        )
    }

    fn by_fields(
        first_column: Option<Column<'static>>,
        identifier: Vec<Column<'static>>,
        order_directive: OrderDirections,
    ) -> OrderVec<'static> {
        match order_directive.primary_order_by {
            Some(order_by) => {
                let first = first_column.unwrap();
                let size_hint = identifier.len() + 1;

                if !identifier.contains(&first)
                    && order_directive.needs_implicit_id_ordering
                    && !order_by.field.unique()
                {
                    match (order_by.sort_order, order_directive.needs_to_be_reverse_order) {
                        (SortOrder::Ascending, true) => {
                            Self::merge_columns(first.descend(), identifier.into_iter().map(|c| c.descend()), size_hint)
                        }
                        (SortOrder::Descending, true) => {
                            Self::merge_columns(first.ascend(), identifier.into_iter().map(|c| c.descend()), size_hint)
                        }
                        (SortOrder::Ascending, false) => {
                            Self::merge_columns(first.ascend(), identifier.into_iter().map(|c| c.ascend()), size_hint)
                        }
                        (SortOrder::Descending, false) => {
                            Self::merge_columns(first.descend(), identifier.into_iter().map(|c| c.ascend()), size_hint)
                        }
                    }
                } else {
                    match (order_by.sort_order, order_directive.needs_to_be_reverse_order) {
                        (SortOrder::Ascending, true) => vec![first.descend()],
                        (SortOrder::Descending, true) => vec![first.ascend()],
                        (SortOrder::Ascending, false) => vec![first.ascend()],
                        (SortOrder::Descending, false) => vec![first.descend()],
                    }
                }
            }
            None if order_directive.needs_implicit_id_ordering && order_directive.needs_to_be_reverse_order => {
                identifier.into_iter().map(|c| c.descend()).collect()
            }
            None if order_directive.needs_implicit_id_ordering && !order_directive.needs_to_be_reverse_order => {
                identifier.into_iter().map(|c| c.ascend()).collect()
            }
            None => Vec::new(),
        }
    }

    fn merge_columns(
        first: OrderDefinition<'static>,
        rest: impl IntoIterator<Item = OrderDefinition<'static>>,
        size_hint: usize,
    ) -> OrderVec<'static> {
        let mut order_vec = Vec::with_capacity(size_hint);
        order_vec.push(first);

        for col in rest.into_iter() {
            order_vec.push(col);
        }

        order_vec
    }
}
