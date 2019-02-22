use crate::ast::DatabaseValue;

pub type OrderDefinition = (DatabaseValue, Option<Order>);

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
