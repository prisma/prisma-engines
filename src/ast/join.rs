use crate::ast::{ConditionTree, Table};

/// The `JOIN` table and conditions.
#[derive(Debug, PartialEq, Clone)]
pub struct JoinData {
    pub(crate) table: Table,
    pub(crate) conditions: ConditionTree,
}

/// A representation of a `JOIN` statement.
#[derive(Debug, PartialEq, Clone)]
pub enum Join {
    /// Implements an `INNER JOIN` with given `JoinData`.
    Inner(JoinData),
    /// Implements an `LEFT OUTER JOIN` with given `JoinData`.
    LeftOuter(JoinData),
}

/// An item that can be joined.
pub trait Joinable {
    /// Add the `JOIN` conditions.
    ///
    /// ```rust
    /// # use prisma_query::{ast::*, visitor::{Visitor, Sqlite}};
    /// let join_data = "b".on(("b", "id").equals(Column::from(("a", "id"))));
    /// let query = Select::from_table("a").inner_join(join_data);
    /// let (sql, _) = Sqlite::build(query);
    ///
    /// assert_eq!(
    ///     "SELECT `a`.* FROM `a` INNER JOIN `b` ON `b`.`id` = `a`.`id` LIMIT -1",
    ///     sql,
    /// );
    /// ```
    fn on<T>(self, conditions: T) -> JoinData
    where
        T: Into<ConditionTree>;
}

impl<U> Joinable for U
where
    U: Into<Table>,
{
    #[inline]
    fn on<T>(self, conditions: T) -> JoinData
    where
        T: Into<ConditionTree>,
    {
        JoinData {
            table: self.into(),
            conditions: conditions.into(),
        }
    }
}
