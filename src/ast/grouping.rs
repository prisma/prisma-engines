use crate::ast::{Column, DatabaseValue};

pub type GroupByDefinition = (DatabaseValue);

/// A list of definitions for the `GROUP BY` statement
#[derive(Debug, Default, PartialEq, Clone)]
pub struct Grouping(pub Vec<GroupByDefinition>);

impl Grouping {
    #[doc(hidden)]
    pub fn append(mut self, value: GroupByDefinition) -> Self {
        self.0.push(value);
        self
    }

    #[inline]
    pub fn new(values: Vec<GroupByDefinition>) -> Self {
        Self(values)
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

/// An item that can be used in the `GROUP BY` statement
pub trait Groupable
where
    Self: Sized,
{
    /// Group by `self`
    fn group(self) -> GroupByDefinition;
}

/// Convert the value into a group by definition.
pub trait IntoGroupByDefinition {
    fn into_group_by_definition(self) -> GroupByDefinition;
}

impl<'a> IntoGroupByDefinition for &'a str {
    #[inline]
    fn into_group_by_definition(self) -> GroupByDefinition {
        let column: Column = self.into();
        (column.into())
    }
}

impl IntoGroupByDefinition for Column {
    #[inline]
    fn into_group_by_definition(self) -> GroupByDefinition {
        (self.into())
    }
}

impl IntoGroupByDefinition for GroupByDefinition {
    #[inline]
    fn into_group_by_definition(self) -> GroupByDefinition {
        self
    }
}

impl Groupable for Column {
    #[inline]
    fn group(self) -> GroupByDefinition {
        (self.into())
    }
}

impl<'a> Groupable for &'a str {
    #[inline]
    fn group(self) -> GroupByDefinition {
        let column: Column = self.into();
        column.group()
    }
}
