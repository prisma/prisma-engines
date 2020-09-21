use std::borrow::Cow;

use super::SelectQuery;

/// A builder for a common table expression (CTE) statement, to be used in the
/// `WITH` block of a `SELECT` statement.
///
/// See [`Select#with`] for usage.
///
/// [`Select#with`]: struct.Select.html#method.with
#[derive(Debug, PartialEq, Clone)]
pub struct CommonTableExpression<'a> {
    pub(crate) identifier: Cow<'a, str>,
    pub(crate) columns: Vec<Cow<'a, str>>,
    pub(crate) selection: SelectQuery<'a>,
}

impl<'a> CommonTableExpression<'a> {
    pub(crate) fn new(identifier: impl Into<Cow<'a, str>>, selection: impl Into<SelectQuery<'a>>) -> Self {
        Self {
            identifier: identifier.into(),
            columns: Vec::new(),
            selection: selection.into(),
        }
    }

    /// Selects a named value from the nested expresion. The statement selects
    /// everything if this method is never called.
    pub fn column(mut self, column: impl Into<Cow<'a, str>>) -> Self {
        self.columns.push(column.into());
        self
    }
}

pub trait IntoCommonTableExpression<'a> {
    /// Conversion into a common table expression, used together
    /// with the [`Select#with`] method.
    ///
    /// [`Select#with`]: struct.Select.html#method.with
    fn into_cte(self, identifier: impl Into<Cow<'a, str>>) -> CommonTableExpression<'a>
    where
        Self: Into<SelectQuery<'a>>,
    {
        CommonTableExpression::new(identifier, self)
    }
}
