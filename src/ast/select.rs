use crate::ast::*;

/// A builder for a `SELECT` statement.
#[derive(Debug, PartialEq, Clone, Default)]
pub struct Select<'a> {
    pub(crate) table: Option<Box<Table<'a>>>,
    pub(crate) columns: Vec<Expression<'a>>,
    pub(crate) conditions: Option<ConditionTree<'a>>,
    pub(crate) ordering: Ordering<'a>,
    pub(crate) grouping: Grouping<'a>,
    pub(crate) having: Option<ConditionTree<'a>>,
    pub(crate) limit: Option<Value<'a>>,
    pub(crate) offset: Option<Value<'a>>,
    pub(crate) joins: Vec<Join<'a>>,
}

impl<'a> From<Select<'a>> for Expression<'a> {
    fn from(sel: Select<'a>) -> Expression<'a> {
        Expression {
            kind: ExpressionKind::Select(Box::new(sel)),
            alias: None,
        }
    }
}

impl<'a> From<Select<'a>> for Query<'a> {
    fn from(sel: Select<'a>) -> Query<'a> {
        Query::Select(Box::new(sel))
    }
}

impl<'a> Select<'a> {
    /// Creates a new `SELECT` statement for the given table.
    ///
    /// ```rust
    /// # use quaint::{ast::*, visitor::{Visitor, Sqlite}};
    /// let query = Select::from_table("users");
    /// let (sql, _) = Sqlite::build(query);
    ///
    /// assert_eq!("SELECT `users`.* FROM `users`", sql);
    /// ```
    ///
    /// The table can be in multiple parts, defining the database.
    ///
    /// ```rust
    /// # use quaint::{ast::*, visitor::{Visitor, Sqlite}};
    /// let query = Select::from_table(("crm", "users"));
    /// let (sql, _) = Sqlite::build(query);
    ///
    /// assert_eq!("SELECT `crm`.`users`.* FROM `crm`.`users`", sql);
    /// ```
    ///
    /// Selecting from a nested `SELECT`.
    ///
    /// ```rust
    /// # use quaint::{ast::*, visitor::{Visitor, Sqlite}};
    /// let select = Table::from(Select::default().value(1)).alias("num");
    /// let query = Select::from_table(select.alias("num"));
    /// let (sql, params) = Sqlite::build(query);
    ///
    /// assert_eq!("SELECT `num`.* FROM (SELECT ?) AS `num`", sql);
    /// assert_eq!(vec![Value::from(1)], params);
    /// ```
    ///
    /// Selecting from a set of values.
    ///
    /// ```rust
    /// # use quaint::{ast::*, visitor::{Visitor, Sqlite}};
    /// # use quaint::values;
    /// let expected_sql = "SELECT `vals`.* FROM (VALUES (?,?),(?,?)) AS `vals`";
    /// let values = Table::from(values!((1, 2), (3, 4))).alias("vals");
    /// let query = Select::from_table(values);
    /// let (sql, params) = Sqlite::build(query);
    ///
    /// assert_eq!(expected_sql, sql);
    /// assert_eq!(
    ///     vec![
    ///         Value::Integer(1),
    ///         Value::Integer(2),
    ///         Value::Integer(3),
    ///         Value::Integer(4),
    ///     ],
    ///     params
    /// );
    /// ```
    pub fn from_table<T>(table: T) -> Self
    where
        T: Into<Table<'a>>,
    {
        Select {
            table: Some(Box::new(table.into())),
            ..Default::default()
        }
    }

    /// Selects a static value as the column.
    ///
    /// ```rust
    /// # use quaint::{ast::*, visitor::{Visitor, Sqlite}};
    /// let query = Select::default().value(1);
    /// let (sql, params) = Sqlite::build(query);
    ///
    /// assert_eq!("SELECT ?", sql);
    /// assert_eq!(vec![Value::from(1)], params);
    /// ```
    ///
    /// Creating a qualified asterisk to a joined table:
    ///
    /// ```rust
    /// # use quaint::{col, val, ast::*, visitor::{Visitor, Sqlite}};
    /// let join = "dogs".on(("dogs", "slave_id").equals(Column::from(("cats", "master_id"))));
    ///
    /// let query = Select::from_table("cats")
    ///     .value(Table::from("cats").asterisk())
    ///     .value(col!("dogs", "age") - val!(4))
    ///     .inner_join(join);
    ///
    /// let (sql, params) = Sqlite::build(query);
    ///
    /// assert_eq!(
    ///     "SELECT `cats`.*, (`dogs`.`age` - ?) FROM `cats` INNER JOIN `dogs` ON `dogs`.`slave_id` = `cats`.`master_id`",
    ///     sql
    /// );
    ///
    /// assert_eq!(vec![Value::from(4)], params);
    /// ```
    pub fn value<T>(mut self, value: T) -> Self
    where
        T: Into<Expression<'a>>,
    {
        self.columns.push(value.into());
        self
    }

    /// Adds a column to be selected.
    ///
    /// ```rust
    /// # use quaint::{ast::*, visitor::{Visitor, Sqlite}};
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
        T: Into<Column<'a>>,
    {
        self.columns.push(column.into().into());
        self
    }

    /// A bulk method to select multiple values.
    ///
    /// ```rust
    /// # use quaint::{ast::*, visitor::{Visitor, Sqlite}};
    /// let query = Select::from_table("users").columns(vec!["foo", "bar"]);
    /// let (sql, _) = Sqlite::build(query);
    ///
    /// assert_eq!("SELECT `foo`, `bar` FROM `users`", sql);
    /// ```
    pub fn columns<T, C>(mut self, columns: T) -> Self
    where
        T: IntoIterator<Item = C>,
        C: Into<Column<'a>>,
    {
        self.columns = columns.into_iter().map(|c| c.into().into()).collect();
        self
    }

    /// Adds `WHERE` conditions to the query, replacing the previous conditions.
    /// See [Comparable](trait.Comparable.html#required-methods) for more
    /// examples.
    ///
    /// ```rust
    /// # use quaint::{ast::*, visitor::{Visitor, Sqlite}};
    /// let query = Select::from_table("users").so_that("foo".equals("bar"));
    /// let (sql, params) = Sqlite::build(query);
    ///
    /// assert_eq!("SELECT `users`.* FROM `users` WHERE `foo` = ?", sql);
    ///
    /// assert_eq!(vec![
    ///    Value::from("bar"),
    /// ], params);
    /// ```
    pub fn so_that<T>(mut self, conditions: T) -> Self
    where
        T: Into<ConditionTree<'a>>,
    {
        self.conditions = Some(conditions.into());
        self
    }

    /// Adds an additional `WHERE` condition to the query combining the possible
    /// previous condition with `AND`. See
    /// [Comparable](trait.Comparable.html#required-methods) for more examples.
    ///
    /// ```rust
    /// # use quaint::{ast::*, visitor::{Visitor, Sqlite}};
    /// let query = Select::from_table("users")
    ///     .so_that("foo".equals("bar"))
    ///     .and_where("lol".equals("wtf"));
    ///
    /// let (sql, params) = Sqlite::build(query);
    ///
    /// assert_eq!("SELECT `users`.* FROM `users` WHERE (`foo` = ? AND `lol` = ?)", sql);
    ///
    /// assert_eq!(vec![
    ///    Value::from("bar"),
    ///    Value::from("wtf"),
    /// ], params);
    /// ```
    pub fn and_where<T>(mut self, conditions: T) -> Self
    where
        T: Into<ConditionTree<'a>>,
    {
        match self.conditions {
            Some(previous) => {
                self.conditions = Some(previous.and(conditions.into()));
                self
            }
            None => self.so_that(conditions),
        }
    }

    /// Adds an additional `WHERE` condition to the query combining the possible
    /// previous condition with `OR`. See
    /// [Comparable](trait.Comparable.html#required-methods) for more examples.
    ///
    /// ```rust
    /// # use quaint::{ast::*, visitor::{Visitor, Sqlite}};
    /// let query = Select::from_table("users")
    ///     .so_that("foo".equals("bar"))
    ///     .or_where("lol".equals("wtf"));
    ///
    /// let (sql, params) = Sqlite::build(query);
    ///
    /// assert_eq!("SELECT `users`.* FROM `users` WHERE (`foo` = ? OR `lol` = ?)", sql);
    ///
    /// assert_eq!(vec![
    ///    Value::from("bar"),
    ///    Value::from("wtf"),
    /// ], params);
    /// ```
    pub fn or_where<T>(mut self, conditions: T) -> Self
    where
        T: Into<ConditionTree<'a>>,
    {
        match self.conditions {
            Some(previous) => {
                self.conditions = Some(previous.or(conditions.into()));
                self
            }
            None => self.so_that(conditions),
        }
    }

    /// Adds `INNER JOIN` clause to the query.
    ///
    /// ```rust
    /// # use quaint::{ast::*, visitor::{Visitor, Sqlite}};
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
        J: Into<JoinData<'a>>,
    {
        self.joins.push(Join::Inner(join.into()));
        self
    }

    /// Adds `LEFT JOIN` clause to the query.
    ///
    /// ```rust
    /// # use quaint::{ast::*, visitor::{Visitor, Sqlite}};
    /// let join = "posts".alias("p").on(("p", "visible").equals(true));
    /// let query = Select::from_table("users").left_join(join);
    /// let (sql, params) = Sqlite::build(query);
    ///
    /// assert_eq!(
    ///     "SELECT `users`.* FROM `users` LEFT JOIN `posts` AS `p` ON `p`.`visible` = ?",
    ///     sql
    /// );
    ///
    /// assert_eq!(
    ///     vec![
    ///         Value::from(true),
    ///     ],
    ///     params
    /// );
    /// ```
    pub fn left_join<J>(mut self, join: J) -> Self
    where
        J: Into<JoinData<'a>>,
    {
        self.joins.push(Join::Left(join.into()));
        self
    }

    /// Adds `RIGHT JOIN` clause to the query.
    ///
    /// ```rust
    /// # use quaint::{ast::*, visitor::{Visitor, Sqlite}};
    /// let join = "posts".alias("p").on(("p", "visible").equals(true));
    /// let query = Select::from_table("users").right_join(join);
    /// let (sql, params) = Sqlite::build(query);
    ///
    /// assert_eq!(
    ///     "SELECT `users`.* FROM `users` RIGHT JOIN `posts` AS `p` ON `p`.`visible` = ?",
    ///     sql
    /// );
    ///
    /// assert_eq!(
    ///     vec![
    ///         Value::from(true),
    ///     ],
    ///     params
    /// );
    /// ```
    pub fn right_join<J>(mut self, join: J) -> Self
    where
        J: Into<JoinData<'a>>,
    {
        self.joins.push(Join::Right(join.into()));
        self
    }

    /// Adds `FULL JOIN` clause to the query.
    ///
    /// ```rust
    /// # use quaint::{ast::*, visitor::{Visitor, Sqlite}};
    /// let join = "posts".alias("p").on(("p", "visible").equals(true));
    /// let query = Select::from_table("users").full_join(join);
    /// let (sql, params) = Sqlite::build(query);
    ///
    /// assert_eq!(
    ///     "SELECT `users`.* FROM `users` FULL JOIN `posts` AS `p` ON `p`.`visible` = ?",
    ///     sql
    /// );
    ///
    /// assert_eq!(
    ///     vec![
    ///         Value::from(true),
    ///     ],
    ///     params
    /// );
    /// ```
    pub fn full_join<J>(mut self, join: J) -> Self
    where
        J: Into<JoinData<'a>>,
    {
        self.joins.push(Join::Full(join.into()));
        self
    }

    /// Adds an ordering to the `ORDER BY` section.
    ///
    /// ```rust
    /// # use quaint::{ast::*, visitor::{Visitor, Sqlite}};
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
        T: IntoOrderDefinition<'a>,
    {
        self.ordering = self.ordering.append(value.into_order_definition());
        self
    }

    /// Adds a grouping to the `GROUP BY` section.
    ///
    /// This does not check if the grouping is actually valid in respect to aggregated columns.
    ///
    /// ```rust
    /// # use quaint::{ast::*, visitor::{Visitor, Sqlite}};
    /// let query = Select::from_table("users").column("foo").column("bar")
    ///     .group_by("foo")
    ///     .group_by("bar");
    ///
    /// let (sql, _) = Sqlite::build(query);
    ///
    /// assert_eq!("SELECT `foo`, `bar` FROM `users` GROUP BY `foo`, `bar`", sql);
    pub fn group_by<T>(mut self, value: T) -> Self
    where
        T: IntoGroupByDefinition<'a>,
    {
        self.grouping = self.grouping.append(value.into_group_by_definition());
        self
    }

    /// Adds group conditions to a query. Should be combined together with a
    /// [group_by](struct.Select.html#method.group_by) statement.
    ///
    /// ```rust
    /// # use quaint::{ast::*, visitor::{Visitor, Sqlite}};
    /// let query = Select::from_table("users").column("foo").column("bar")
    ///     .group_by("foo")
    ///     .having("foo".greater_than(100));
    ///
    /// let (sql, params) = Sqlite::build(query);
    ///
    /// assert_eq!("SELECT `foo`, `bar` FROM `users` GROUP BY `foo` HAVING `foo` > ?", sql);
    /// assert_eq!(vec![Value::from(100)], params);
    pub fn having<T>(mut self, conditions: T) -> Self
    where
        T: Into<ConditionTree<'a>>,
    {
        self.having = Some(conditions.into());
        self
    }

    /// Sets the `LIMIT` value.
    ///
    /// ```rust
    /// # use quaint::{ast::*, visitor::{Visitor, Sqlite}};
    /// let query = Select::from_table("users").limit(10);
    /// let (sql, params) = Sqlite::build(query);
    ///
    /// assert_eq!("SELECT `users`.* FROM `users` LIMIT ?", sql);
    /// assert_eq!(vec![Value::from(10)], params);
    pub fn limit(mut self, limit: usize) -> Self {
        self.limit = Some(Value::from(limit));
        self
    }

    /// Sets the `OFFSET` value.
    ///
    /// ```rust
    /// # use quaint::{ast::*, visitor::{Visitor, Sqlite}};
    /// let query = Select::from_table("users").offset(10);
    /// let (sql, params) = Sqlite::build(query);
    ///
    /// assert_eq!("SELECT `users`.* FROM `users` LIMIT ? OFFSET ?", sql);
    /// assert_eq!(vec![Value::from(-1), Value::from(10)], params);
    pub fn offset(mut self, offset: usize) -> Self {
        self.offset = Some(Value::from(offset));
        self
    }
}
