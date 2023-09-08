use crate::ast::{ConditionTree, Table};

/// The `JOIN` table and conditions.
#[derive(Debug, PartialEq, Clone)]
pub struct JoinData<'a> {
    pub(crate) table: Table<'a>,
    pub(crate) conditions: ConditionTree<'a>,
}

impl<'a> JoinData<'a> {
    /// Implement a join with no conditions.
    pub fn all_from(table: impl Into<Table<'a>>) -> Self {
        Self {
            table: table.into(),
            conditions: ConditionTree::NoCondition,
        }
    }
}

impl<'a, T> From<T> for JoinData<'a>
where
    T: Into<Table<'a>>,
{
    fn from(table: T) -> Self {
        Self::all_from(table)
    }
}

/// A representation of a `JOIN` statement.
#[derive(Debug, PartialEq, Clone)]
pub enum Join<'a> {
    /// Implements an `INNER JOIN` with given `JoinData`.
    Inner(JoinData<'a>),
    /// Implements an `LEFT JOIN` with given `JoinData`.
    Left(JoinData<'a>),
    /// Implements an `RIGHT JOIN` with given `JoinData`.
    Right(JoinData<'a>),
    /// Implements an `FULL JOIN` with given `JoinData`.
    Full(JoinData<'a>),
}

/// An item that can be joined.
pub trait Joinable<'a> {
    /// Add the `JOIN` conditions.
    ///
    /// ```rust
    /// # use quaint::{ast::*, visitor::{Visitor, Sqlite}};
    /// # fn main() -> Result<(), quaint::error::Error> {
    /// let join_data = "b".on(("b", "id").equals(Column::from(("a", "id"))));
    /// let query = Select::from_table("a").inner_join(join_data);
    /// let (sql, _) = Sqlite::build(query)?;
    ///
    /// assert_eq!(
    ///     "SELECT `a`.* FROM `a` INNER JOIN `b` ON `b`.`id` = `a`.`id`",
    ///     sql,
    /// );
    /// # Ok(())
    /// # }
    /// ```
    fn on<T>(self, conditions: T) -> JoinData<'a>
    where
        T: Into<ConditionTree<'a>>;
}

impl<'a, U> Joinable<'a> for U
where
    U: Into<Table<'a>>,
{
    fn on<T>(self, conditions: T) -> JoinData<'a>
    where
        T: Into<ConditionTree<'a>>,
    {
        JoinData {
            table: self.into(),
            conditions: conditions.into(),
        }
    }
}

impl<'a> Joinable<'a> for JoinData<'a> {
    fn on<T>(self, conditions: T) -> JoinData<'a>
    where
        T: Into<ConditionTree<'a>>,
    {
        let conditions = match self.conditions {
            ConditionTree::NoCondition => conditions.into(),
            cond => cond.and(conditions.into()),
        };

        JoinData {
            table: self.table,
            conditions,
        }
    }
}
