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
    /// Selects a named value from the nested expresion. The statement selects
    /// everything if this method is never called.
    pub fn column(mut self, column: impl Into<Cow<'a, str>>) -> Self {
        self.columns.push(column.into());
        self
    }
}

/// Conversion into a common table expression.
///
/// Used together with the [`Select#with`] method.
///
/// [`Select#with`]: struct.Select.html#method.with
pub trait IntoCommonTableExpression<'a> {
    fn into_cte(self, identifier: impl Into<Cow<'a, str>>) -> CommonTableExpression<'a>
    where
        Self: Into<SelectQuery<'a>>,
    {
        CommonTableExpression {
            identifier: identifier.into(),
            columns: Vec::new(),
            selection: self.into(),
        }
    }
}
