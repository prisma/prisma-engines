use super::Function;
use crate::ast::{Column, Expression};

/// Defines a grouping for the `GROUP BY` statement.
pub type GroupByDefinition<'a> = Expression<'a>;

/// A list of definitions for the `GROUP BY` statement
#[derive(Debug, Default, PartialEq, Clone)]
pub struct Grouping<'a>(pub Vec<GroupByDefinition<'a>>);

impl<'a> Grouping<'a> {
    #[doc(hidden)]
    pub fn append(mut self, value: GroupByDefinition<'a>) -> Self {
        self.0.push(value);
        self
    }

    pub fn new(values: Vec<GroupByDefinition<'a>>) -> Self {
        Self(values)
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

/// An item that can be used in the `GROUP BY` statement
pub trait Groupable<'a>
where
    Self: Sized,
{
    /// Group by `self`
    fn group(self) -> GroupByDefinition<'a>;
}

/// Convert the value into a group by definition.
pub trait IntoGroupByDefinition<'a> {
    fn into_group_by_definition(self) -> GroupByDefinition<'a>;
}

impl<'a> IntoGroupByDefinition<'a> for &'a str {
    fn into_group_by_definition(self) -> GroupByDefinition<'a> {
        let column: Column = self.into();
        column.into()
    }
}

impl<'a> IntoGroupByDefinition<'a> for (&'a str, &'a str) {
    fn into_group_by_definition(self) -> GroupByDefinition<'a> {
        let column: Column = self.into();
        column.into()
    }
}

impl<'a> IntoGroupByDefinition<'a> for Column<'a> {
    fn into_group_by_definition(self) -> GroupByDefinition<'a> {
        self.into()
    }
}

impl<'a> IntoGroupByDefinition<'a> for Function<'a> {
    fn into_group_by_definition(self) -> GroupByDefinition<'a> {
        self.into()
    }
}

impl<'a> IntoGroupByDefinition<'a> for GroupByDefinition<'a> {
    fn into_group_by_definition(self) -> GroupByDefinition<'a> {
        self
    }
}

impl<'a> Groupable<'a> for Column<'a> {
    fn group(self) -> GroupByDefinition<'a> {
        self.into()
    }
}

impl<'a> Groupable<'a> for &'a str {
    fn group(self) -> GroupByDefinition<'a> {
        Column::from(self).group()
    }
}
