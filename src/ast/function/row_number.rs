use crate::ast::{Column, IntoOrderDefinition, Ordering};

#[derive(Debug, Default, Clone, PartialEq)]
pub struct Over {
    pub(crate) ordering: Ordering,
    pub(crate) partitioning: Vec<Column>,
}

impl Over {
    pub fn is_empty(&self) -> bool {
        self.ordering.is_empty() && self.partitioning.is_empty()
    }
}

#[derive(Debug, Default, Clone, PartialEq)]
pub struct RowNumber {
    pub(crate) over: Over,
}

impl RowNumber {
    /// Define the order of the row number. Is the row order if not set.
    pub fn order_by<T>(mut self, value: T) -> Self
    where
        T: IntoOrderDefinition,
    {
        self.over.ordering = self.over.ordering.append(value.into_order_definition());
        self
    }

    /// Define the partitioning of the row number
    pub fn partition_by<T>(mut self, partition: T) -> Self
    where
        T: Into<Column>,
    {
        self.over.partitioning.push(partition.into());
        self
    }
}

/// A number from 1 to n in specified order
///
/// ```rust
/// # use prisma_query::{ast::*, visitor::{Visitor, Sqlite}};
/// let fun = Function::from(row_number().order_by("created_at").partition_by("name"));
///
/// let query = Select::from_table("users")
///     .column("id")
///     .value(fun.alias("num"));
///
/// let (sql, _) = Sqlite::build(query);
///
/// assert_eq!(
///     "SELECT `id`, ROW_NUMBER() OVER(PARTITION BY `name` ORDER BY `created_at`) AS `num` FROM `users` LIMIT -1",
///     sql
/// );
/// ```
#[inline]
pub fn row_number() -> RowNumber {
    RowNumber::default()
}
