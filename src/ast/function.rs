use crate::ast::{DatabaseValue, IntoOrderDefinition, Ordering};

/// A database function definition
#[derive(Debug, Clone, PartialEq)]
pub enum Function {
    RowNumber(RowNumber),
}

#[derive(Debug, Default, Clone, PartialEq)]
pub struct RowNumber {
    pub ordering: Ordering,
    pub alias: Option<String>,
}

impl RowNumber {
    /// Define the order of the row number. Is the row order if not set.
    pub fn over<T>(mut self, value: T) -> Self
    where
        T: IntoOrderDefinition,
    {
        self.ordering = self.ordering.append(value.into_order_definition());
        self
    }
}

/// A number from 1 to n in specified order
///
/// ```rust
/// # use prisma_query::{ast::*, visitor::{Visitor, Sqlite}};
/// let query = Select::from("users")
///     .column("id")
///     .value(row_number().over("created_at"));
/// let (sql, _) = Sqlite::build(query);
///
/// assert_eq!(

///     "SELECT `id`, ROW_NUMBER() OVER(ORDER BY `created_at`) FROM `users` LIMIT -1",
///     sql
/// );
/// ```
pub fn row_number() -> RowNumber {
    RowNumber::default()
}

impl From<RowNumber> for DatabaseValue {
    fn from(rn: RowNumber) -> DatabaseValue {
        Function::RowNumber(rn).into()
    }
}
