use crate::ast::{Column, DatabaseValue};

pub type GroupByDefinition<'a> = DatabaseValue<'a>;

/// A list of definitions for the `GROUP BY` statement
#[derive(Debug, Default, PartialEq, Clone)]
pub struct Grouping<'a>(pub Vec<GroupByDefinition<'a>>);

impl<'a> Grouping<'a> {
    #[doc(hidden)]
    pub fn append(mut self, value: GroupByDefinition<'a>) -> Self {
        self.0.push(value);
        self
    }

    #[inline]
    pub fn new(values: Vec<GroupByDefinition<'a>>) -> Self {
        Self(values)
    }

    #[inline]
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
    #[inline]
    fn into_group_by_definition(self) -> GroupByDefinition<'a> {
        let column: Column = self.into();
        column.into()
    }
}

impl<'a> IntoGroupByDefinition<'a> for (&'a str, &'a str) {
    #[inline]
    fn into_group_by_definition(self) -> GroupByDefinition<'a> {
        let column: Column = self.into();
        column.into()
    }
}

impl<'a> IntoGroupByDefinition<'a> for Column<'a> {
    #[inline]
    fn into_group_by_definition(self) -> GroupByDefinition<'a> {
        self.into()
    }
}

impl<'a> IntoGroupByDefinition<'a> for GroupByDefinition<'a> {
    #[inline]
    fn into_group_by_definition(self) -> GroupByDefinition<'a> {
        self
    }
}

impl<'a> Groupable<'a> for Column<'a> {
    #[inline]
    fn group(self) -> GroupByDefinition<'a> {
        self.into()
    }
}

impl<'a> Groupable<'a> for &'a str {
    #[inline]
    fn group(self) -> GroupByDefinition<'a> {
        Column::from(self).group()
    }
}
