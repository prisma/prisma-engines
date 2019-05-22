use crate::ast::*;

/// A builder for a `SELECT` statement.
#[derive(Debug, PartialEq, Clone, Default)]
pub struct Select {
    pub(crate) table: Option<Box<Table>>,
    pub(crate) columns: Vec<DatabaseValue>,
    pub(crate) conditions: Option<ConditionTree>,
    pub(crate) ordering: Ordering,
    pub(crate) grouping: Grouping,
    pub(crate) limit: Option<ParameterizedValue>,
    pub(crate) offset: Option<ParameterizedValue>,
    pub(crate) joins: Vec<Join>,
}

impl Into<DatabaseValue> for Select {
    #[inline]
    fn into(self) -> DatabaseValue {
        DatabaseValue::Select(self)
    }
}

impl From<Select> for Query {
    #[inline]
    fn from(sel: Select) -> Query {
        Query::Select(sel)
    }
}

impl Select {
    /// Creates a new `SELECT` statement for the given table.
    ///
    /// ```rust
    /// # use prisma_query::{ast::*, visitor::{Visitor, Sqlite}};
    /// let query = Select::from_table("users");
    /// let (sql, _) = Sqlite::build(query);
    ///
    /// assert_eq!("SELECT `users`.* FROM `users`", sql);
    /// ```
    ///
    /// The table can be in multiple parts, defining the database.
    ///
    /// ```rust
    /// # use prisma_query::{ast::*, visitor::{Visitor, Sqlite}};
    /// let query = Select::from_table(("crm", "users"));
    /// let (sql, _) = Sqlite::build(query);
    ///
    /// assert_eq!("SELECT `crm`.`users`.* FROM `crm`.`users`", sql);
    /// ```
    ///
    /// It is also possible to use a nested `SELECT`.
    ///
    /// ```rust
    /// # use prisma_query::{ast::*, visitor::{Visitor, Sqlite}};
    /// let select = Table::from(Select::default().value(1)).alias("num");
    /// let query = Select::from_table(select.alias("num"));
    /// let (sql, params) = Sqlite::build(query);
    ///
    /// assert_eq!("SELECT `num`.* FROM (SELECT ?) AS `num`", sql);
    /// assert_eq!(vec![ParameterizedValue::from(1)], params);
    /// ```
    #[inline]
    pub fn from_table<T>(table: T) -> Self
    where
        T: Into<Table>,
    {
        Select {
            table: Some(Box::new(table.into())),
            ..Default::default()
        }
    }

    /// Selects a static value as the column.
    ///
    /// ```rust
    /// # use prisma_query::{ast::*, visitor::{Visitor, Sqlite}};
    /// let query = Select::default().value(1);
    /// let (sql, params) = Sqlite::build(query);
    ///
    /// assert_eq!("SELECT ?", sql);
    /// assert_eq!(vec![ParameterizedValue::from(1)], params);
    /// ```
    ///
    /// Creating a qualified asterisk to a joined table:
    ///
    /// ```rust
    /// # use prisma_query::{ast::*, visitor::{Visitor, Sqlite}};
    /// let join = "dogs".on(("dogs", "slave_id").equals(Column::from(("cats", "master_id"))));
    ///
    /// let query = Select::from_table("cats")
    ///     .value(Table::from("cats").asterisk())
    ///     .value(Table::from("dogs").asterisk())
    ///     .inner_join(join);
    ///
    /// let (sql, _) = Sqlite::build(query);
    ///
    /// assert_eq!(
    ///     "SELECT `cats`.*, `dogs`.* FROM `cats` INNER JOIN `dogs` ON `dogs`.`slave_id` = `cats`.`master_id`",
    ///     sql
    /// );
    /// ```
    pub fn value<T>(mut self, value: T) -> Self
    where
        T: Into<DatabaseValue>,
    {
        self.columns.push(value.into());
        self
    }

    /// Adds a column to be selected.
    ///
    /// ```rust
    /// # use prisma_query::{ast::*, visitor::{Visitor, Sqlite}};
    /// let query = Select::from_table("users")
    ///     .column("name")
    ///     .column(("users", "id"))
    ///     .column((("crm", "users"), "foo"));
    ///
    /// let (sql, _) = Sqlite::build(query);
    ///
    /// assert_eq!("SELECT `name`, `users`.`id`, `crm`.`users`.`foo` FROM `users`", sql);
    /// ```
    pub fn column<T>(mut self, column: T) -> Self
    where
        T: Into<Column>,
    {
        self.columns.push(column.into().into());
        self
    }

    /// A bulk method to select multiple values.
    ///
    /// ```rust
    /// # use prisma_query::{ast::*, visitor::{Visitor, Sqlite}};
    /// let query = Select::from_table("users").columns(vec!["foo", "bar"]);
    /// let (sql, params) = Sqlite::build(query);
    ///
    /// assert_eq!("SELECT ?, ? FROM `users`", sql);
    ///
    /// assert_eq!(vec![
    ///    ParameterizedValue::from("foo"),
    ///    ParameterizedValue::from("bar"),
    /// ], params);
    /// ```
    pub fn columns<T>(mut self, columns: Vec<T>) -> Self
    where
        T: Into<DatabaseValue>,
    {
        self.columns = columns.into_iter().map(|c| c.into()).collect();
        self
    }

    /// Adds `WHERE` conditions to the query. See
    /// [Comparable](trait.Comparable.html#required-methods) for more examples.
    ///
    /// ```rust
    /// # use prisma_query::{ast::*, visitor::{Visitor, Sqlite}};
    /// let query = Select::from_table("users").so_that("foo".equals("bar"));
    /// let (sql, params) = Sqlite::build(query);
    ///
    /// assert_eq!("SELECT `users`.* FROM `users` WHERE `foo` = ?", sql);
    ///
    /// assert_eq!(vec![
    ///    ParameterizedValue::from("bar"),
    /// ], params);
    /// ```
    pub fn so_that<T>(mut self, conditions: T) -> Self
    where
        T: Into<ConditionTree>,
    {
        self.conditions = Some(conditions.into());
        self
    }

    /// Adds `INNER JOIN` clause to the query.
    ///
    /// ```rust
    /// # use prisma_query::{ast::*, visitor::{Visitor, Sqlite}};
    /// let join = "posts".alias("p").on(("p", "user_id").equals(Column::from(("users", "id"))));
    /// let query = Select::from_table("users").inner_join(join);
    /// let (sql, _) = Sqlite::build(query);
    ///
    /// assert_eq!(
    ///     "SELECT `users`.* FROM `users` INNER JOIN `posts` AS `p` ON `p`.`user_id` = `users`.`id`",
    ///     sql
    /// );
    /// ```
    pub fn inner_join<J>(mut self, join: J) -> Self
    where
        J: Into<JoinData>,
    {
        self.joins.push(Join::Inner(join.into()));
        self
    }

    /// Adds `LEFT OUTER JOIN` clause to the query.
    ///
    /// ```rust
    /// # use prisma_query::{ast::*, visitor::{Visitor, Sqlite}};
    /// let join = "posts".alias("p").on(("p", "visible").equals(true));
    /// let query = Select::from_table("users").left_outer_join(join);
    /// let (sql, params) = Sqlite::build(query);
    ///
    /// assert_eq!(
    ///     "SELECT `users`.* FROM `users` LEFT OUTER JOIN `posts` AS `p` ON `p`.`visible` = ?",
    ///     sql
    /// );
    ///
    /// assert_eq!(
    ///     vec![
    ///         ParameterizedValue::from(true),
    ///     ],
    ///     params
    /// );
    /// ```
    pub fn left_outer_join<J>(mut self, join: J) -> Self
    where
        J: Into<JoinData>,
    {
        self.joins.push(Join::LeftOuter(join.into()));
        self
    }

    /// Adds an ordering to the `ORDER BY` section.
    ///
    /// ```rust
    /// # use prisma_query::{ast::*, visitor::{Visitor, Sqlite}};
    /// let query = Select::from_table("users")
    ///     .order_by("foo")
    ///     .order_by("baz".ascend())
    ///     .order_by("bar".descend());
    ///
    /// let (sql, _) = Sqlite::build(query);
    ///
    /// assert_eq!("SELECT `users`.* FROM `users` ORDER BY `foo`, `baz` ASC, `bar` DESC", sql);
    pub fn order_by<T>(mut self, value: T) -> Self
    where
        T: IntoOrderDefinition,
    {
        self.ordering = self.ordering.append(value.into_order_definition());
        self
    }

    /// Adds a grouping to the `GROUP BY` section.
    ///
    /// This does not check if the grouping is actually valid in respect to aggregated columns.
    ///
    /// ```rust
    /// # use prisma_query::{ast::*, visitor::{Visitor, Sqlite}};
    /// let query = Select::from_table("users").column("foo").column("bar")
    ///     .group_by("foo")
    ///     .group_by("bar");
    ///
    /// let (sql, _) = Sqlite::build(query);
    ///
    /// assert_eq!("SELECT `foo`, `bar` FROM `users` GROUP BY `foo`, `bar`", sql);
    pub fn group_by<T>(mut self, value: T) -> Self
    where
        T: IntoGroupByDefinition,
    {
        self.grouping = self.grouping.append(value.into_group_by_definition());
        self
    }

    /// Sets the `LIMIT` value.
    ///
    /// ```rust
    /// # use prisma_query::{ast::*, visitor::{Visitor, Sqlite}};
    /// let query = Select::from_table("users").limit(10);
    /// let (sql, params) = Sqlite::build(query);
    ///
    /// assert_eq!("SELECT `users`.* FROM `users` LIMIT ?", sql);
    /// assert_eq!(vec![ParameterizedValue::from(10)], params);
    pub fn limit(mut self, limit: usize) -> Self {
        self.limit = Some(ParameterizedValue::from(limit));
        self
    }

    /// Sets the `OFFSET` value.
    ///
    /// ```rust
    /// # use prisma_query::{ast::*, visitor::{Visitor, Sqlite}};
    /// let query = Select::from_table("users").offset(10);
    /// let (sql, params) = Sqlite::build(query);
    ///
    /// assert_eq!("SELECT `users`.* FROM `users` LIMIT ? OFFSET ?", sql);
    /// assert_eq!(vec![ParameterizedValue::from(-1), ParameterizedValue::from(10)], params);
    pub fn offset(mut self, offset: usize) -> Self {
        self.offset = Some(ParameterizedValue::from(offset));
        self
    }
}
