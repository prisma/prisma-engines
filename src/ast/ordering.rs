use crate::ast::{Column, Expression};

/// Defines ordering for an `ORDER BY` statement.
pub type OrderDefinition<'a> = (Expression<'a>, Option<Order>);

/// A list of definitions for the `ORDER BY` statement.
#[derive(Debug, Default, PartialEq, Clone)]
pub struct Ordering<'a>(pub Vec<OrderDefinition<'a>>);

impl<'a> Ordering<'a> {
    #[doc(hidden)]
    pub fn append(mut self, value: OrderDefinition<'a>) -> Self {
        self.0.push(value);
        self
    }

    pub fn new(values: Vec<OrderDefinition<'a>>) -> Self {
        Self(values)
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

/// The ordering direction
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Order {
    /// Ascending
    Asc,
    /// Descending
    Desc,
    /// Ascending Nulls First
    AscNullsFirst,
    /// Ascending Nulls Last
    AscNullsLast,
    /// Descending Nulls First
    DescNullsFirst,
    /// Descending Nulls Last
    DescNullsLast,
}

/// An item that can be used in the `ORDER BY` statement
pub trait Orderable<'a>
where
    Self: Sized,
{
    /// Order by `self` in the given order
    fn order(self, order: Option<Order>) -> OrderDefinition<'a>;

    /// Change the order to `ASC`
    fn ascend(self) -> OrderDefinition<'a> {
        self.order(Some(Order::Asc))
    }

    /// Change the order to `DESC`
    fn descend(self) -> OrderDefinition<'a> {
        self.order(Some(Order::Desc))
    }

    /// Change the order to `ASC NULLS FIRST`
    fn ascend_nulls_first(self) -> OrderDefinition<'a> {
        self.order(Some(Order::AscNullsFirst))
    }

    /// Change the order to `ASC NULLS LAST`
    fn ascend_nulls_last(self) -> OrderDefinition<'a> {
        self.order(Some(Order::AscNullsLast))
    }

    /// Change the order to `DESC NULLS FIRST`
    fn descend_nulls_first(self) -> OrderDefinition<'a> {
        self.order(Some(Order::DescNullsFirst))
    }

    /// Change the order to `ASC NULLS LAST`
    fn descend_nulls_last(self) -> OrderDefinition<'a> {
        self.order(Some(Order::DescNullsLast))
    }
}

/// Convert the value into an order definition with order item and direction
pub trait IntoOrderDefinition<'a> {
    fn into_order_definition(self) -> OrderDefinition<'a>;
}

impl<'a> IntoOrderDefinition<'a> for &'a str {
    fn into_order_definition(self) -> OrderDefinition<'a> {
        let column: Column<'a> = self.into();
        (column.into(), None)
    }
}

impl<'a> IntoOrderDefinition<'a> for Column<'a> {
    fn into_order_definition(self) -> OrderDefinition<'a> {
        (self.into(), None)
    }
}

impl<'a> IntoOrderDefinition<'a> for OrderDefinition<'a> {
    fn into_order_definition(self) -> OrderDefinition<'a> {
        self
    }
}

impl<'a> Orderable<'a> for Column<'a> {
    fn order(self, order: Option<Order>) -> OrderDefinition<'a> {
        (self.into(), order)
    }
}

impl<'a> Orderable<'a> for &'a str {
    fn order(self, order: Option<Order>) -> OrderDefinition<'a> {
        let column: Column<'a> = self.into();
        column.order(order)
    }
}

impl<'a> Orderable<'a> for (&'a str, &'a str) {
    fn order(self, order: Option<Order>) -> OrderDefinition<'a> {
        let column: Column<'a> = self.into();
        column.order(order)
    }
}
