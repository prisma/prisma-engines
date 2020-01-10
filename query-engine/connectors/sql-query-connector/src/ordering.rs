use connector_interface::OrderDirections;
use prisma_models::*;
use quaint::ast::*;

pub type OrderVec<'a> = Vec<(DatabaseValue<'a>, Option<Order>)>;

pub struct Ordering;

/// Tooling for generating orderings for different query types.
impl Ordering {
    pub fn for_model(model: &ModelRef, order_directive: OrderDirections) -> OrderVec<'static> {
        Self::by_fields(
            order_directive
                .primary_order_by
                .as_ref()
                .map(|oby| oby.field.as_column()),
            model.identifier().as_columns().collect(),
            order_directive,
        )
    }

    /*
    pub fn internal<C>(second_field: C, order_directive: OrderDirections) -> OrderVec<'static>
    where
        C: Into<Column<'static>>,
    {
        Self::by_fields(
            order_directive
                .primary_order_by
                .as_ref()
                .map(|oby| oby.field.as_column()),
            vec![second_field.into()],
            order_directive,
        )
    }

    pub fn aliased_internal(
        alias: &'static str,
        secondary_alias: &'static str,
        secondary_field: &'static str,
        order_directive: OrderDirections,
    ) -> OrderVec<'static> {
        Self::by_fields(
            order_directive
                .primary_order_by
                .as_ref()
                .map(|oby| (alias.to_string(), oby.field.db_name().to_string()).into()),
            vec![secondary_alias.into(), secondary_field.into()],
            order_directive,
        )
    }
    */

    fn by_fields(
        first_column: Option<Column<'static>>,
        identifier: Vec<Column<'static>>,
        order_directive: OrderDirections,
    ) -> OrderVec<'static> {
        match order_directive.primary_order_by {
            Some(order_by) => {
                let first = first_column.unwrap();
                let size_hint = identifier.len() + 1;

                if !identifier.contains(&first) && order_directive.needs_implicit_id_ordering && !order_by.field.unique() {
                    match (order_by.sort_order, order_directive.needs_to_be_reverse_order) {
                        (SortOrder::Ascending, true) => {
                            Self::merge_columns(
                                first.descend(),
                                identifier.into_iter().map(|c| c.descend()),
                                size_hint,
                            )
                        }
                        (SortOrder::Descending, true) => {
                            Self::merge_columns(
                                first.ascend(),
                                identifier.into_iter().map(|c| c.descend()),
                                size_hint,
                            )
                        }
                        (SortOrder::Ascending, false) => {
                            Self::merge_columns(
                                first.ascend(),
                                identifier.into_iter().map(|c| c.ascend()),
                                size_hint,
                            )
                        }
                        (SortOrder::Descending, false) => {
                            Self::merge_columns(
                                first.descend(),
                                identifier.into_iter().map(|c| c.ascend()),
                                size_hint,
                            )
                        },
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
        size_hint: usize
    ) -> OrderVec<'static> {
        let mut order_vec = Vec::with_capacity(size_hint);
        order_vec.push(first);

        for col in rest.into_iter() {
            order_vec.push(col);
        }

        order_vec
    }
}
