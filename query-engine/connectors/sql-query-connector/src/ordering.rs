use connector_interface::OrderDirections;
use prisma_models::*;
use quaint::ast::*;

pub type OrderVec<'a> = Vec<(DatabaseValue<'a>, Option<Order>)>;

pub struct Ordering;

/// Tooling for generating orderings for different query types.
impl Ordering {
    pub fn for_model(model: ModelRef, order_directive: OrderDirections) -> OrderVec<'static> {
        Self::by_fields(
            order_directive
                .primary_order_by
                .as_ref()
                .map(|oby| oby.field.as_column()),
            model.fields().id().as_column(),
            order_directive,
        )
    }

    pub fn internal<C>(second_field: C, order_directive: OrderDirections) -> OrderVec<'static>
    where
        C: Into<Column<'static>>,
    {
        Self::by_fields(
            order_directive
                .primary_order_by
                .as_ref()
                .map(|oby| oby.field.as_column()),
            second_field.into(),
            order_directive,
        )
    }

    pub fn aliased_internal(
        alias: &str,
        secondary_alias: &str,
        secondary_field: &str,
        order_directive: OrderDirections,
    ) -> OrderVec<'static> {
        Self::by_fields(
            order_directive
                .primary_order_by
                .as_ref()
                .map(|oby| (alias.to_string(), oby.field.db_name().to_string()).into()),
            (secondary_alias.to_string(), secondary_field.to_string()).into(),
            order_directive,
        )
    }

    fn by_fields(
        first_column: Option<Column<'static>>,
        second_column: Column<'static>,
        order_directive: OrderDirections,
    ) -> OrderVec<'static> {
        match order_directive.primary_order_by {
            Some(order_by) => {
                let first = first_column.unwrap();
                if first != second_column && order_directive.needs_implicit_id_ordering && !order_by.field.unique() {
                    match (order_by.sort_order, order_directive.needs_to_be_reverse_order) {
                        (SortOrder::Ascending, true) => vec![first.descend(), second_column.descend()],
                        (SortOrder::Descending, true) => vec![first.ascend(), second_column.descend()],
                        (SortOrder::Ascending, false) => vec![first.ascend(), second_column.ascend()],
                        (SortOrder::Descending, false) => vec![first.descend(), second_column.ascend()],
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
                vec![second_column.descend()]
            }
            None if order_directive.needs_implicit_id_ordering && !order_directive.needs_to_be_reverse_order => {
                vec![second_column.ascend()]
            }
            None => vec![],
        }
    }
}
