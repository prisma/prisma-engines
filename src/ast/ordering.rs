use crate::{
    ast::DatabaseValue,
    visitor::{Destination, Visitor},
};

pub type OrderDefinition = (DatabaseValue, Option<Order>);

#[derive(Debug, Default, PartialEq, Clone)]
pub struct Ordering(Vec<OrderDefinition>);

impl Ordering {
    #[doc(hidden)]
    pub fn append(mut self, value: OrderDefinition) -> Self {
        self.0.push(value);
        self
    }

    pub fn new(values: Vec<OrderDefinition>) -> Self {
        Self(values)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Order {
    Ascending,
    Descending,
}

pub trait Orderable
where
    Self: Sized,
{
    fn order(self, order: Option<Order>) -> OrderDefinition;

    fn ascend(self) -> OrderDefinition {
        self.order(Some(Order::Ascending))
    }

    fn descend(self) -> OrderDefinition {
        self.order(Some(Order::Descending))
    }
}

impl Destination for Ordering {
    fn visit(&self, visitor: &mut Visitor) {
        visitor.visit_ordering(self);
    }
}
