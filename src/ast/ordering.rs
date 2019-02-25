use crate::ast::{Column, DatabaseValue};

pub type OrderDefinition = (DatabaseValue, Option<Order>);

/// A list of definitions for the `ORDER BY` statement
#[derive(Debug, Default, PartialEq, Clone)]
pub struct Ordering(pub Vec<OrderDefinition>);

impl Ordering {
    #[doc(hidden)]
    pub fn append(mut self, value: OrderDefinition) -> Self {
        self.0.push(value);
        self
    }

    pub fn new(values: Vec<OrderDefinition>) -> Self {
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
}

/// An item that can be used in the `ORDER BY` statement
pub trait Orderable
where
    Self: Sized,
{
    /// Order by `self` in the given order
    fn order(self, order: Option<Order>) -> OrderDefinition;

    /// Change the order to `ASC`
    fn ascend(self) -> OrderDefinition {
        self.order(Some(Order::Asc))
    }

    /// Change the order to `DESC`
    fn descend(self) -> OrderDefinition {
        self.order(Some(Order::Desc))
    }
}

/// Convert the value into an order definition with order item and direction
pub trait IntoOrderDefinition {
    fn into_order_definition(self) -> OrderDefinition;
}

impl<'a> IntoOrderDefinition for &'a str {
    fn into_order_definition(self) -> OrderDefinition {
        let column: Column = self.into();
        (column.into(), None)
    }
}

impl IntoOrderDefinition for Column {
    fn into_order_definition(self) -> OrderDefinition {
        (self.into(), None)
    }
}

impl IntoOrderDefinition for OrderDefinition {
    fn into_order_definition(self) -> OrderDefinition {
        self
    }
}

impl Orderable for Column {
    fn order(self, order: Option<Order>) -> OrderDefinition {
        (self.into(), order)
    }
}

impl<'a> Orderable for &'a str {
    fn order(self, order: Option<Order>) -> OrderDefinition {
        let column: Column = self.into();
        column.order(order)
    }
}
