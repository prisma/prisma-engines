use crate::ast::*;

/// A builder for a `SELECT` statement.
#[derive(Debug, PartialEq, Clone, Default)]
pub struct Select {
    pub table: Option<Table>,
    pub columns: Vec<DatabaseValue>,
    pub conditions: Option<ConditionTree>,
    pub ordering: Ordering,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
    pub joins: Vec<Join>,
}

impl Into<DatabaseValue> for Select {
    fn into(self) -> DatabaseValue {
        DatabaseValue::Select(self)
    }
}

impl Into<Query> for Select {
    fn into(self) -> Query {
        Query::Select(self)
    }
}

impl Select {
    /// Creates a new `SELECT` statement from the given table.
    ///
    /// ```rust
    /// # use prisma_query::{ast::*, visitor::{Visitor, Sqlite}};
    /// # fn main() {
    /// let query = Select::from("users");
    /// let (sql, _) = Sqlite::build(query);
    ///
    /// assert_eq!("SELECT * FROM `users` LIMIT -1", sql);
    /// # }
    /// ```
    ///
    /// The table can be in multiple parts, defining the database.
    ///
    /// ```rust
    /// # use prisma_query::{ast::*, visitor::{Visitor, Sqlite}};
    ///
    /// # fn main() {
    /// let query = Select::from(("crm", "users"));
    /// let (sql, _) = Sqlite::build(query);
    ///
    /// assert_eq!("SELECT * FROM `crm`.`users` LIMIT -1", sql);
    /// # }
    /// ```
    pub fn from<T>(table: T) -> Self
    where
        T: Into<Table>,
    {
        Select {
            table: Some(table.into()),
            ..Default::default()
        }
    }

    /// Selects a static value as the column.
    ///
    /// ```rust
    /// # use prisma_query::{ast::*, visitor::{Visitor, Sqlite}};
    /// # fn main() {
    /// let query = Select::default().value(1);
    /// let (sql, params) = Sqlite::build(query);
    ///
    /// assert_eq!("SELECT ?", sql);
    /// assert_eq!(vec![ParameterizedValue::Integer(1)], params);
    /// # }
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
    /// # fn main() {
    /// let query = Select::from("users")
    ///     .column("name")
    ///     .column(("users", "id"))
    ///     .column(("crm", "users", "foo"));
    ///
    /// let (sql, _) = Sqlite::build(query);
    ///
    /// assert_eq!("SELECT `name`, `users`.`id`, `crm`.`users`.`foo` FROM `users` LIMIT -1", sql);
    /// # }
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
    /// # fn main() {
    /// let query = Select::from("users").columns(vec!["foo", "bar"]);
    /// let (sql, params) = Sqlite::build(query);
    ///
    /// assert_eq!("SELECT ?, ? FROM `users` LIMIT -1", sql);
    /// assert_eq!(vec![
    ///    ParameterizedValue::Text("foo".to_string()),
    ///    ParameterizedValue::Text("bar".to_string())
    /// ], params);
    /// # }
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
    /// # fn main() {
    /// let query = Select::from("users").so_that("foo".equals("bar"));
    /// let (sql, params) = Sqlite::build(query);
    ///
    /// assert_eq!("SELECT * FROM `users` WHERE `foo` = ? LIMIT -1", sql);
    /// assert_eq!(vec![ParameterizedValue::Text("bar".to_string())], params);
    /// # }
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
    /// # fn main() {
    /// let join = "posts".alias("p").on(("p", "user_id").equals(Column::from(("users", "id"))));
    /// let query = Select::from("users").inner_join(join);
    /// let (sql, _) = Sqlite::build(query);
    ///
    /// assert_eq!("SELECT * FROM `users` INNER JOIN `posts` AS `p` ON `p`.`user_id` = `users`.`id` LIMIT -1", sql);
    /// # }
    /// ```
    pub fn inner_join<J>(mut self, join: J) -> Self
    where
        J: Into<JoinData>,
    {
        self.joins.push(Join::Inner(join.into()));
        self
    }

    /// Adds an ordering to the `ORDER BY` section.
    ///
    /// ```rust
    /// # use prisma_query::{ast::*, visitor::{Visitor, Sqlite}};
    /// # fn main() {
    /// let query = Select::from("users")
    ///     .order_by("foo")
    ///     .order_by("baz".ascend())
    ///     .order_by("bar".descend());
    ///
    /// let (sql, _) = Sqlite::build(query);
    ///
    /// assert_eq!("SELECT * FROM `users` ORDER BY `foo`, `baz` ASC, `bar` DESC LIMIT -1", sql);
    /// # }
    pub fn order_by<T>(mut self, value: T) -> Self
    where
        T: IntoOrderDefinition,
    {
        self.ordering = self.ordering.append(value.into_order_definition());
        self
    }

    /// Sets the `LIMIT` value.
    ///
    /// ```rust
    /// # use prisma_query::{ast::*, visitor::{Visitor, Sqlite}};
    /// # fn main() {
    /// let query = Select::from("users").limit(10);
    /// let (sql, _) = Sqlite::build(query);
    ///
    /// assert_eq!("SELECT * FROM `users` LIMIT 10", sql);
    /// # }
    pub fn limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }

    /// Sets the `OFFSET` value.
    ///
    /// ```rust
    /// # use prisma_query::{ast::*, visitor::{Visitor, Sqlite}};
    /// # fn main() {
    /// let query = Select::from("users").offset(10);
    /// let (sql, _) = Sqlite::build(query);
    ///
    /// assert_eq!("SELECT * FROM `users` LIMIT -1 OFFSET 10", sql);
    /// # }
    pub fn offset(mut self, offset: usize) -> Self {
        self.offset = Some(offset);
        self
    }
}
