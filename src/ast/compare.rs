use super::ExpressionKind;
use crate::ast::{Column, ConditionTree, Expression};
use std::borrow::Cow;

/// For modeling comparison expression
#[derive(Debug, Clone, PartialEq)]
pub enum Compare<'a> {
    /// `left = right`
    Equals(Box<Expression<'a>>, Box<Expression<'a>>),
    /// `left <> right`
    NotEquals(Box<Expression<'a>>, Box<Expression<'a>>),
    /// `left < right`
    LessThan(Box<Expression<'a>>, Box<Expression<'a>>),
    /// `left <= right`
    LessThanOrEquals(Box<Expression<'a>>, Box<Expression<'a>>),
    /// `left > right`
    GreaterThan(Box<Expression<'a>>, Box<Expression<'a>>),
    /// `left >= right`
    GreaterThanOrEquals(Box<Expression<'a>>, Box<Expression<'a>>),
    /// `left IN (..)`
    In(Box<Expression<'a>>, Box<Expression<'a>>),
    /// `left NOT IN (..)`
    NotIn(Box<Expression<'a>>, Box<Expression<'a>>),
    /// `left LIKE %..%`
    Like(Box<Expression<'a>>, Cow<'a, str>),
    /// `left NOT LIKE %..%`
    NotLike(Box<Expression<'a>>, Cow<'a, str>),
    /// `left LIKE ..%`
    BeginsWith(Box<Expression<'a>>, Cow<'a, str>),
    /// `left NOT LIKE ..%`
    NotBeginsWith(Box<Expression<'a>>, Cow<'a, str>),
    /// `left LIKE %..`
    EndsInto(Box<Expression<'a>>, Cow<'a, str>),
    /// `left NOT LIKE %..`
    NotEndsInto(Box<Expression<'a>>, Cow<'a, str>),
    /// `value IS NULL`
    Null(Box<Expression<'a>>),
    /// `value IS NOT NULL`
    NotNull(Box<Expression<'a>>),
    /// `value` BETWEEN `left` AND `right`
    Between(Box<Expression<'a>>, Box<Expression<'a>>, Box<Expression<'a>>),
    /// `value` NOT BETWEEN `left` AND `right`
    NotBetween(Box<Expression<'a>>, Box<Expression<'a>>, Box<Expression<'a>>),
}

impl<'a> From<Compare<'a>> for ConditionTree<'a> {
    fn from(cmp: Compare<'a>) -> Self {
        ConditionTree::single(Expression::from(cmp))
    }
}

impl<'a> From<Compare<'a>> for Expression<'a> {
    fn from(cmp: Compare<'a>) -> Self {
        Expression {
            kind: ExpressionKind::Compare(cmp),
            alias: None,
        }
    }
}

/// An item that can be compared against other values in the database.
pub trait Comparable<'a> {
    /// Tests if both sides are the same value.
    ///
    /// ```rust
    /// # use quaint::{ast::*, visitor::{Visitor, Sqlite}};
    /// let query = Select::from_table("users").so_that("foo".equals("bar"));
    /// let (sql, params) = Sqlite::build(query);
    ///
    /// assert_eq!("SELECT `users`.* FROM `users` WHERE `foo` = ?", sql);
    ///
    /// assert_eq!(
    ///     vec![
    ///         Value::from("bar"),
    ///     ],
    ///     params
    /// );
    /// ```
    fn equals<T>(self, comparison: T) -> Compare<'a>
    where
        T: Into<Expression<'a>>;

    /// Tests if both sides are not the same value.
    ///
    /// ```rust
    /// # use quaint::{ast::*, visitor::{Visitor, Sqlite}};
    /// let query = Select::from_table("users").so_that("foo".not_equals("bar"));
    /// let (sql, params) = Sqlite::build(query);
    ///
    /// assert_eq!("SELECT `users`.* FROM `users` WHERE `foo` <> ?", sql);
    ///
    /// assert_eq!(
    ///     vec![
    ///         Value::from("bar"),
    ///     ],
    ///     params
    /// );
    /// ```
    fn not_equals<T>(self, comparison: T) -> Compare<'a>
    where
        T: Into<Expression<'a>>;

    /// Tests if the left side is smaller than the right side.
    ///
    /// ```rust
    /// # use quaint::{ast::*, visitor::{Visitor, Sqlite}};
    /// let query = Select::from_table("users").so_that("foo".less_than(10));
    /// let (sql, params) = Sqlite::build(query);
    ///
    /// assert_eq!("SELECT `users`.* FROM `users` WHERE `foo` < ?", sql);
    ///
    /// assert_eq!(
    ///     vec![
    ///         Value::from(10),
    ///     ],
    ///     params
    /// );
    /// ```
    fn less_than<T>(self, comparison: T) -> Compare<'a>
    where
        T: Into<Expression<'a>>;

    /// Tests if the left side is smaller than the right side or the same.
    ///
    /// ```rust
    /// # use quaint::{ast::*, visitor::{Visitor, Sqlite}};
    /// let query = Select::from_table("users").so_that("foo".less_than_or_equals(10));
    /// let (sql, params) = Sqlite::build(query);
    ///
    /// assert_eq!("SELECT `users`.* FROM `users` WHERE `foo` <= ?", sql);
    ///
    /// assert_eq!(
    ///     vec![
    ///         Value::from(10),
    ///     ],
    ///     params
    /// );
    /// ```
    fn less_than_or_equals<T>(self, comparison: T) -> Compare<'a>
    where
        T: Into<Expression<'a>>;

    /// Tests if the left side is bigger than the right side.
    ///
    /// ```rust
    /// # use quaint::{ast::*, visitor::{Visitor, Sqlite}};
    /// let query = Select::from_table("users").so_that("foo".greater_than(10));
    /// let (sql, params) = Sqlite::build(query);
    ///
    /// assert_eq!("SELECT `users`.* FROM `users` WHERE `foo` > ?", sql);
    ///
    /// assert_eq!(
    ///     vec![
    ///         Value::from(10),
    ///     ],
    ///     params
    /// );
    /// ```
    fn greater_than<T>(self, comparison: T) -> Compare<'a>
    where
        T: Into<Expression<'a>>;

    /// Tests if the left side is bigger than the right side or the same.
    ///
    /// ```rust
    /// # use quaint::{ast::*, visitor::{Visitor, Sqlite}};
    /// let query = Select::from_table("users").so_that("foo".greater_than_or_equals(10));
    /// let (sql, params) = Sqlite::build(query);
    ///
    /// assert_eq!("SELECT `users`.* FROM `users` WHERE `foo` >= ?", sql);
    ///
    /// assert_eq!(
    ///     vec![
    ///         Value::from(10),
    ///     ],
    ///     params
    /// );
    /// ```
    fn greater_than_or_equals<T>(self, comparison: T) -> Compare<'a>
    where
        T: Into<Expression<'a>>;

    /// Tests if the left side is included in the right side collection.
    ///
    /// ```rust
    /// # use quaint::{ast::*, visitor::{Visitor, Sqlite}};
    /// let query = Select::from_table("users").so_that("foo".in_selection(vec![1, 2]));
    /// let (sql, params) = Sqlite::build(query);
    ///
    /// assert_eq!("SELECT `users`.* FROM `users` WHERE `foo` IN (?,?)", sql);
    /// assert_eq!(vec![
    ///     Value::Integer(1),
    ///     Value::Integer(2),
    /// ], params);
    /// ```
    fn in_selection<T>(self, selection: T) -> Compare<'a>
    where
        T: Into<Expression<'a>>;

    /// Tests if the left side is not included in the right side collection.
    ///
    /// ```rust
    /// # use quaint::{ast::*, visitor::{Visitor, Sqlite}};
    /// let query = Select::from_table("users").so_that("foo".not_in_selection(vec![1, 2]));
    /// let (sql, params) = Sqlite::build(query);
    ///
    /// assert_eq!("SELECT `users`.* FROM `users` WHERE `foo` NOT IN (?,?)", sql);
    ///
    /// assert_eq!(vec![
    ///     Value::Integer(1),
    ///     Value::Integer(2),
    /// ], params);
    /// ```
    fn not_in_selection<T>(self, selection: T) -> Compare<'a>
    where
        T: Into<Expression<'a>>;

    /// Tests if the left side includes the right side string.
    ///
    /// ```rust
    /// # use quaint::{ast::*, visitor::{Visitor, Sqlite}};
    /// let query = Select::from_table("users").so_that("foo".like("bar"));
    /// let (sql, params) = Sqlite::build(query);
    ///
    /// assert_eq!("SELECT `users`.* FROM `users` WHERE `foo` LIKE ?", sql);
    ///
    /// assert_eq!(
    ///     vec![
    ///         Value::from("%bar%"),
    ///     ],
    ///     params
    /// );
    /// ```
    fn like<T>(self, pattern: T) -> Compare<'a>
    where
        T: Into<Cow<'a, str>>;

    /// Tests if the left side does not include the right side string.
    ///
    /// ```rust
    /// # use quaint::{ast::*, visitor::{Visitor, Sqlite}};
    /// let query = Select::from_table("users").so_that("foo".not_like("bar"));
    /// let (sql, params) = Sqlite::build(query);
    ///
    /// assert_eq!("SELECT `users`.* FROM `users` WHERE `foo` NOT LIKE ?", sql);
    ///
    /// assert_eq!(
    ///     vec![
    ///         Value::from("%bar%"),
    ///     ],
    ///     params
    /// );
    /// ```
    fn not_like<T>(self, pattern: T) -> Compare<'a>
    where
        T: Into<Cow<'a, str>>;

    /// Tests if the left side starts with the right side string.
    ///
    /// ```rust
    /// # use quaint::{ast::*, visitor::{Visitor, Sqlite}};
    /// let query = Select::from_table("users").so_that("foo".begins_with("bar"));
    /// let (sql, params) = Sqlite::build(query);
    ///
    /// assert_eq!("SELECT `users`.* FROM `users` WHERE `foo` LIKE ?", sql);
    ///
    /// assert_eq!(
    ///     vec![
    ///         Value::from("bar%"),
    ///     ],
    ///     params
    /// );
    /// ```
    fn begins_with<T>(self, pattern: T) -> Compare<'a>
    where
        T: Into<Cow<'a, str>>;

    /// Tests if the left side doesn't start with the right side string.
    ///
    /// ```rust
    /// # use quaint::{ast::*, visitor::{Visitor, Sqlite}};
    /// let query = Select::from_table("users").so_that("foo".not_begins_with("bar"));
    /// let (sql, params) = Sqlite::build(query);
    ///
    /// assert_eq!("SELECT `users`.* FROM `users` WHERE `foo` NOT LIKE ?", sql);
    ///
    /// assert_eq!(
    ///     vec![
    ///         Value::from("bar%"),
    ///     ],
    ///     params
    /// );
    /// ```
    fn not_begins_with<T>(self, pattern: T) -> Compare<'a>
    where
        T: Into<Cow<'a, str>>;

    /// Tests if the left side ends into the right side string.
    ///
    /// ```rust
    /// # use quaint::{ast::*, visitor::{Visitor, Sqlite}};
    /// let query = Select::from_table("users").so_that("foo".ends_into("bar"));
    /// let (sql, params) = Sqlite::build(query);
    ///
    /// assert_eq!("SELECT `users`.* FROM `users` WHERE `foo` LIKE ?", sql);
    ///
    /// assert_eq!(
    ///     vec![
    ///         Value::from("%bar"),
    ///     ],
    ///     params
    /// );
    /// ```
    fn ends_into<T>(self, pattern: T) -> Compare<'a>
    where
        T: Into<Cow<'a, str>>;

    /// Tests if the left side does not end into the right side string.
    ///
    /// ```rust
    /// # use quaint::{ast::*, visitor::{Visitor, Sqlite}};
    /// let query = Select::from_table("users").so_that("foo".not_ends_into("bar"));
    /// let (sql, params) = Sqlite::build(query);
    ///
    /// assert_eq!("SELECT `users`.* FROM `users` WHERE `foo` NOT LIKE ?", sql);
    ///
    /// assert_eq!(
    ///     vec![
    ///         Value::from("%bar"),
    ///     ],
    ///     params
    /// );
    /// ```
    fn not_ends_into<T>(self, pattern: T) -> Compare<'a>
    where
        T: Into<Cow<'a, str>>;

    /// Tests if the left side is `NULL`.
    ///
    /// ```rust
    /// # use quaint::{ast::*, visitor::{Visitor, Sqlite}};
    /// let query = Select::from_table("users").so_that("foo".is_null());
    /// let (sql, _) = Sqlite::build(query);
    ///
    /// assert_eq!("SELECT `users`.* FROM `users` WHERE `foo` IS NULL", sql);
    /// ```
    fn is_null(self) -> Compare<'a>;

    /// Tests if the left side is not `NULL`.
    ///
    /// ```rust
    /// # use quaint::{ast::*, visitor::{Visitor, Sqlite}};
    /// let query = Select::from_table("users").so_that("foo".is_not_null());
    /// let (sql, _) = Sqlite::build(query);
    ///
    /// assert_eq!("SELECT `users`.* FROM `users` WHERE `foo` IS NOT NULL", sql);
    /// ```
    fn is_not_null(self) -> Compare<'a>;

    /// Tests if the value is between two given values.
    ///
    /// ```rust
    /// # use quaint::{ast::*, visitor::{Visitor, Sqlite}};
    /// let query = Select::from_table("users").so_that("foo".between(420, 666));
    /// let (sql, params) = Sqlite::build(query);
    ///
    /// assert_eq!("SELECT `users`.* FROM `users` WHERE `foo` BETWEEN ? AND ?", sql);
    ///
    /// assert_eq!(vec![
    ///     Value::Integer(420),
    ///     Value::Integer(666),
    /// ], params);
    /// ```
    fn between<T, V>(self, left: T, right: V) -> Compare<'a>
    where
        T: Into<Expression<'a>>,
        V: Into<Expression<'a>>;

    /// Tests if the value is not between two given values.
    ///
    /// ```rust
    /// # use quaint::{ast::*, visitor::{Visitor, Sqlite}};
    /// let query = Select::from_table("users").so_that("foo".not_between(420, 666));
    /// let (sql, params) = Sqlite::build(query);
    ///
    /// assert_eq!("SELECT `users`.* FROM `users` WHERE `foo` NOT BETWEEN ? AND ?", sql);
    ///
    /// assert_eq!(vec![
    ///     Value::Integer(420),
    ///     Value::Integer(666),
    /// ], params);
    /// ```
    fn not_between<T, V>(self, left: T, right: V) -> Compare<'a>
    where
        T: Into<Expression<'a>>,
        V: Into<Expression<'a>>;
}

impl<'a, U> Comparable<'a> for U
where
    U: Into<Column<'a>>,
{
    fn equals<T>(self, comparison: T) -> Compare<'a>
    where
        T: Into<Expression<'a>>,
    {
        let col: Column<'a> = self.into();
        let val: Expression<'a> = col.into();

        val.equals(comparison)
    }

    fn not_equals<T>(self, comparison: T) -> Compare<'a>
    where
        T: Into<Expression<'a>>,
    {
        let col: Column<'a> = self.into();
        let val: Expression<'a> = col.into();
        val.not_equals(comparison)
    }

    fn less_than<T>(self, comparison: T) -> Compare<'a>
    where
        T: Into<Expression<'a>>,
    {
        let col: Column<'a> = self.into();
        let val: Expression<'a> = col.into();
        val.less_than(comparison)
    }

    fn less_than_or_equals<T>(self, comparison: T) -> Compare<'a>
    where
        T: Into<Expression<'a>>,
    {
        let col: Column<'a> = self.into();
        let val: Expression<'a> = col.into();
        val.less_than_or_equals(comparison)
    }

    fn greater_than<T>(self, comparison: T) -> Compare<'a>
    where
        T: Into<Expression<'a>>,
    {
        let col: Column<'a> = self.into();
        let val: Expression<'a> = col.into();
        val.greater_than(comparison)
    }

    fn greater_than_or_equals<T>(self, comparison: T) -> Compare<'a>
    where
        T: Into<Expression<'a>>,
    {
        let col: Column<'a> = self.into();
        let val: Expression<'a> = col.into();
        val.greater_than_or_equals(comparison)
    }

    fn in_selection<T>(self, selection: T) -> Compare<'a>
    where
        T: Into<Expression<'a>>,
    {
        let col: Column<'a> = self.into();
        let val: Expression<'a> = col.into();
        val.in_selection(selection)
    }

    fn not_in_selection<T>(self, selection: T) -> Compare<'a>
    where
        T: Into<Expression<'a>>,
    {
        let col: Column<'a> = self.into();
        let val: Expression<'a> = col.into();
        val.not_in_selection(selection)
    }

    fn like<T>(self, pattern: T) -> Compare<'a>
    where
        T: Into<Cow<'a, str>>,
    {
        let col: Column<'a> = self.into();
        let val: Expression<'a> = col.into();
        val.like(pattern)
    }

    fn not_like<T>(self, pattern: T) -> Compare<'a>
    where
        T: Into<Cow<'a, str>>,
    {
        let col: Column<'a> = self.into();
        let val: Expression<'a> = col.into();
        val.not_like(pattern)
    }

    fn begins_with<T>(self, pattern: T) -> Compare<'a>
    where
        T: Into<Cow<'a, str>>,
    {
        let col: Column<'a> = self.into();
        let val: Expression<'a> = col.into();
        val.begins_with(pattern)
    }

    fn not_begins_with<T>(self, pattern: T) -> Compare<'a>
    where
        T: Into<Cow<'a, str>>,
    {
        let col: Column<'a> = self.into();
        let val: Expression<'a> = col.into();
        val.not_begins_with(pattern)
    }

    fn ends_into<T>(self, pattern: T) -> Compare<'a>
    where
        T: Into<Cow<'a, str>>,
    {
        let col: Column<'a> = self.into();
        let val: Expression<'a> = col.into();
        val.ends_into(pattern)
    }

    fn not_ends_into<T>(self, pattern: T) -> Compare<'a>
    where
        T: Into<Cow<'a, str>>,
    {
        let col: Column<'a> = self.into();
        let val: Expression<'a> = col.into();
        val.not_ends_into(pattern)
    }

    fn is_null(self) -> Compare<'a> {
        let col: Column<'a> = self.into();
        let val: Expression<'a> = col.into();
        val.is_null()
    }

    fn is_not_null(self) -> Compare<'a> {
        let col: Column<'a> = self.into();
        let val: Expression<'a> = col.into();
        val.is_not_null()
    }

    fn between<T, V>(self, left: T, right: V) -> Compare<'a>
    where
        T: Into<Expression<'a>>,
        V: Into<Expression<'a>>,
    {
        let col: Column<'a> = self.into();
        let val: Expression<'a> = col.into();
        val.between(left, right)
    }

    fn not_between<T, V>(self, left: T, right: V) -> Compare<'a>
    where
        T: Into<Expression<'a>>,
        V: Into<Expression<'a>>,
    {
        let col: Column<'a> = self.into();
        let val: Expression<'a> = col.into();
        val.not_between(left, right)
    }
}
