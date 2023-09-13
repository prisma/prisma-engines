use super::ExpressionKind;
use crate::ast::{Column, ConditionTree, Expression};
use std::borrow::Cow;

/// For modeling comparison expressions.
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
    Like(Box<Expression<'a>>, Box<Expression<'a>>),
    /// `left NOT LIKE %..%`
    NotLike(Box<Expression<'a>>, Box<Expression<'a>>),
    /// `value IS NULL`
    Null(Box<Expression<'a>>),
    /// `value IS NOT NULL`
    NotNull(Box<Expression<'a>>),
    /// `value` BETWEEN `left` AND `right`
    Between(Box<Expression<'a>>, Box<Expression<'a>>, Box<Expression<'a>>),
    /// `value` NOT BETWEEN `left` AND `right`
    NotBetween(Box<Expression<'a>>, Box<Expression<'a>>, Box<Expression<'a>>),
    /// Raw comparator, allows to use an operator `left <raw> right` as is,
    /// without visitor transformation in between.
    Raw(Box<Expression<'a>>, Cow<'a, str>, Box<Expression<'a>>),
    /// All json related comparators
    #[cfg(any(feature = "postgresql", feature = "mysql"))]
    JsonCompare(JsonCompare<'a>),
    /// `left` @@ to_tsquery(`value`)
    #[cfg(feature = "postgresql")]
    Matches(Box<Expression<'a>>, Cow<'a, str>),
    /// (NOT `left` @@ to_tsquery(`value`))
    #[cfg(feature = "postgresql")]
    NotMatches(Box<Expression<'a>>, Cow<'a, str>),
    /// ANY (`left`)
    #[cfg(feature = "postgresql")]
    Any(Box<Expression<'a>>),
    /// ALL (`left`)
    #[cfg(feature = "postgresql")]
    All(Box<Expression<'a>>),
}

#[derive(Debug, Clone, PartialEq)]
pub enum JsonCompare<'a> {
    ArrayContains(Box<Expression<'a>>, Box<Expression<'a>>),
    ArrayNotContains(Box<Expression<'a>>, Box<Expression<'a>>),
    TypeEquals(Box<Expression<'a>>, JsonType<'a>),
    TypeNotEquals(Box<Expression<'a>>, JsonType<'a>),
}

#[derive(Debug, Clone, PartialEq)]
pub enum JsonType<'a> {
    Array,
    Object,
    String,
    Number,
    Boolean,
    Null,
    ColumnRef(Box<Column<'a>>),
}

impl<'a> From<Column<'a>> for JsonType<'a> {
    fn from(col: Column<'a>) -> Self {
        JsonType::ColumnRef(Box::new(col))
    }
}

impl<'a> Compare<'a> {
    /// Finds a possible `(a,y) IN (SELECT x,z FROM B)`, takes the select out and
    /// converts the comparison into `a IN (SELECT x FROM cte_n where z = y)`.
    ///
    /// Left side means a match and the CTE should be handled, right side is a
    /// no-op.
    #[cfg(feature = "mssql")]
    pub(crate) fn convert_tuple_select_to_cte(
        self,
        level: &mut usize,
    ) -> either::Either<Self, (Self, Vec<super::CommonTableExpression<'a>>)> {
        use super::IntoCommonTableExpression;

        fn convert<'a>(
            row: super::Row<'a>,
            select: super::SelectQuery<'a>,
            mut selected_columns: Vec<String>,
            level: &mut usize,
        ) -> (
            super::Column<'a>,
            super::Select<'a>,
            Vec<super::CommonTableExpression<'a>>,
        ) {
            // Get the columns out from the row.
            let mut cols = row.into_columns();

            // The name of the CTE in the query
            let ident = format!("cte_{level}");

            let (select, ctes) = select.convert_tuple_selects_to_ctes(level);

            let mut combined_ctes = Vec::with_capacity(ctes.len() + 1);
            combined_ctes.push(select.into_cte(ident.clone()));
            combined_ctes.extend(ctes);

            // The left side column of the comparison, `*this* IN (...)`. We can
            // support a single value comparisons in all databases, so we try to
            // find the first value of the tuple, converting the select to hold
            // the rest of the values in its comparison.
            let comp_col = cols.remove(0);

            // The right side `SELECT` of the comparison, replacing the original
            // `SELECT`.  At this point we just select the first column from the
            // original select, changing the `SELECT` into
            // `(SELECT first_col FROM cte_n)`.
            let base_select = super::Select::from_table(ident).column(selected_columns.remove(0));

            // We know we have the same amount of columns on both sides,
            let column_pairs = cols.into_iter().zip(selected_columns);

            // Adding to the new select a condition to filter out the rest of
            // the tuple, so if our tuple is `(a, b) IN (SELECT x, y ..)`, this
            // will then turn into `a IN (SELECT x WHERE b = y)`.
            let inner_select = column_pairs.fold(base_select, |acc, (left_col, right_col)| {
                acc.and_where(right_col.equals(left_col))
            });

            // Now we added one cte, so we must increment the count for the
            // possible other expressions.
            *level += 1;

            // Return the comparison data to the caller.
            (comp_col, inner_select, combined_ctes)
        }

        match self {
            Self::In(left, right) if left.is_row() && right.is_selection() => {
                let row = left.into_row().unwrap();
                let select = right.into_selection().unwrap();
                let selection = select.named_selection();

                if row.len() != selection.len() {
                    let left = Expression::row(row);
                    let right = Expression::selection(select);

                    return either::Either::Left(left.in_selection(right));
                }

                if row.is_only_columns() && row.len() > 1 {
                    let (comp_col, inner_select, ctes) = convert(row, select, selection, level);
                    let cond = comp_col.in_selection(inner_select);

                    either::Either::Right((cond, ctes))
                } else if row.len() == 1 {
                    let left = Expression::row(row);
                    let (select, ctes) = select.convert_tuple_selects_to_ctes(level);

                    let select = Expression::selection(select);
                    let cond = Self::In(Box::new(left), Box::new(select));

                    either::Either::Right((cond, ctes))
                } else {
                    let left = Expression::row(row);
                    let select = Expression::selection(select);
                    let cond = Self::In(Box::new(left), Box::new(select));

                    either::Either::Left(cond)
                }
            }
            Self::In(left, right) if right.is_selection() => {
                let (selection, ctes) = right.into_selection().unwrap().convert_tuple_selects_to_ctes(level);
                let cond = Self::In(left, Box::new(Expression::selection(selection)));

                either::Either::Right((cond, ctes))
            }
            Self::NotIn(left, right) if left.is_row() && right.is_selection() => {
                let row = left.into_row().unwrap();
                let select = right.into_selection().unwrap();
                let selection = select.named_selection();

                if row.len() != selection.len() {
                    let left = Expression::row(row);
                    let right = Expression::selection(select);

                    return either::Either::Left(left.not_in_selection(right));
                }

                if row.is_only_columns() && row.len() > 1 {
                    let (comp_col, inner_select, ctes) = convert(row, select, selection, level);
                    let cond = comp_col.not_in_selection(inner_select);

                    either::Either::Right((cond, ctes))
                } else if row.len() == 1 {
                    let left = Expression::row(row);
                    let (select, ctes) = select.convert_tuple_selects_to_ctes(level);

                    let select = Expression::selection(select);
                    let cond = Self::NotIn(Box::new(left), Box::new(select));

                    either::Either::Right((cond, ctes))
                } else {
                    let left = Expression::row(row);
                    let select = Expression::selection(select);
                    let cond = Self::NotIn(Box::new(left), Box::new(select));

                    either::Either::Left(cond)
                }
            }
            Self::NotIn(left, right) if right.is_selection() => {
                let (selection, ctes) = right.into_selection().unwrap().convert_tuple_selects_to_ctes(level);
                let cond = Self::NotIn(left, Box::new(Expression::selection(selection)));

                either::Either::Right((cond, ctes))
            }
            _ => either::Either::Left(self),
        }
    }
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
    /// # fn main() -> Result<(), quaint::error::Error> {
    /// let query = Select::from_table("users").so_that("foo".equals("bar"));
    /// let (sql, params) = Sqlite::build(query)?;
    ///
    /// assert_eq!("SELECT `users`.* FROM `users` WHERE `foo` = ?", sql);
    ///
    /// assert_eq!(
    ///     vec![
    ///         Value::from("bar"),
    ///     ],
    ///     params
    /// );
    /// # Ok(())
    /// # }
    /// ```
    fn equals<T>(self, comparison: T) -> Compare<'a>
    where
        T: Into<Expression<'a>>;

    /// Tests if both sides are not the same value.
    ///
    /// ```rust
    /// # use quaint::{ast::*, visitor::{Visitor, Sqlite}};
    /// # fn main() -> Result<(), quaint::error::Error> {
    /// let query = Select::from_table("users").so_that("foo".not_equals("bar"));
    /// let (sql, params) = Sqlite::build(query)?;
    ///
    /// assert_eq!("SELECT `users`.* FROM `users` WHERE `foo` <> ?", sql);
    ///
    /// assert_eq!(
    ///     vec![
    ///         Value::from("bar"),
    ///     ],
    ///     params
    /// );
    /// # Ok(())
    /// # }
    /// ```
    fn not_equals<T>(self, comparison: T) -> Compare<'a>
    where
        T: Into<Expression<'a>>;

    /// Tests if the left side is smaller than the right side.
    ///
    /// ```rust
    /// # fn main() -> Result<(), quaint::error::Error> {
    /// # use quaint::{ast::*, visitor::{Visitor, Sqlite}};
    /// let query = Select::from_table("users").so_that("foo".less_than(10));
    /// let (sql, params) = Sqlite::build(query)?;
    ///
    /// assert_eq!("SELECT `users`.* FROM `users` WHERE `foo` < ?", sql);
    ///
    /// assert_eq!(
    ///     vec![
    ///         Value::from(10),
    ///     ],
    ///     params
    /// );
    /// # Ok(())
    /// # }
    /// ```
    fn less_than<T>(self, comparison: T) -> Compare<'a>
    where
        T: Into<Expression<'a>>;

    /// Tests if the left side is smaller than the right side or the same.
    ///
    /// ```rust
    /// # use quaint::{ast::*, visitor::{Visitor, Sqlite}};
    /// # fn main() -> Result<(), quaint::error::Error> {
    /// let query = Select::from_table("users").so_that("foo".less_than_or_equals(10));
    /// let (sql, params) = Sqlite::build(query)?;
    ///
    /// assert_eq!("SELECT `users`.* FROM `users` WHERE `foo` <= ?", sql);
    ///
    /// assert_eq!(
    ///     vec![
    ///         Value::from(10),
    ///     ],
    ///     params
    /// );
    /// # Ok(())
    /// # }
    /// ```
    fn less_than_or_equals<T>(self, comparison: T) -> Compare<'a>
    where
        T: Into<Expression<'a>>;

    /// Tests if the left side is bigger than the right side.
    ///
    /// ```rust
    /// # use quaint::{ast::*, visitor::{Visitor, Sqlite}};
    /// # fn main() -> Result<(), quaint::error::Error> {
    /// let query = Select::from_table("users").so_that("foo".greater_than(10));
    /// let (sql, params) = Sqlite::build(query)?;
    ///
    /// assert_eq!("SELECT `users`.* FROM `users` WHERE `foo` > ?", sql);
    ///
    /// assert_eq!(
    ///     vec![
    ///         Value::from(10),
    ///     ],
    ///     params
    /// );
    /// # Ok(())
    /// # }
    /// ```
    fn greater_than<T>(self, comparison: T) -> Compare<'a>
    where
        T: Into<Expression<'a>>;

    /// Tests if the left side is bigger than the right side or the same.
    ///
    /// ```rust
    /// # use quaint::{ast::*, visitor::{Visitor, Sqlite}};
    /// # fn main() -> Result<(), quaint::error::Error> {
    /// let query = Select::from_table("users").so_that("foo".greater_than_or_equals(10));
    /// let (sql, params) = Sqlite::build(query)?;
    ///
    /// assert_eq!("SELECT `users`.* FROM `users` WHERE `foo` >= ?", sql);
    ///
    /// assert_eq!(
    ///     vec![
    ///         Value::from(10),
    ///     ],
    ///     params
    /// );
    /// # Ok(())
    /// # }
    /// ```
    fn greater_than_or_equals<T>(self, comparison: T) -> Compare<'a>
    where
        T: Into<Expression<'a>>;

    /// Tests if the left side is included in the right side collection.
    ///
    /// ```rust
    /// # use quaint::{ast::*, visitor::{Visitor, Sqlite}};
    /// # fn main() -> Result<(), quaint::error::Error> {
    /// let query = Select::from_table("users").so_that("foo".in_selection(vec![1, 2]));
    /// let (sql, params) = Sqlite::build(query)?;
    ///
    /// assert_eq!("SELECT `users`.* FROM `users` WHERE `foo` IN (?,?)", sql);
    /// assert_eq!(vec![
    ///     Value::from(1),
    ///     Value::from(2),
    /// ], params);
    /// # Ok(())
    /// # }
    /// ```
    fn in_selection<T>(self, selection: T) -> Compare<'a>
    where
        T: Into<Expression<'a>>;

    /// Tests if the left side is not included in the right side collection.
    ///
    /// ```rust
    /// # use quaint::{ast::*, visitor::{Visitor, Sqlite}};
    /// # fn main() -> Result<(), quaint::error::Error> {
    /// let query = Select::from_table("users").so_that("foo".not_in_selection(vec![1, 2]));
    /// let (sql, params) = Sqlite::build(query)?;
    ///
    /// assert_eq!("SELECT `users`.* FROM `users` WHERE `foo` NOT IN (?,?)", sql);
    ///
    /// assert_eq!(vec![
    ///     Value::from(1),
    ///     Value::from(2),
    /// ], params);
    /// # Ok(())
    /// # }
    /// ```
    fn not_in_selection<T>(self, selection: T) -> Compare<'a>
    where
        T: Into<Expression<'a>>;

    /// Tests if the left side includes the right side string.
    ///
    /// ```rust
    /// # use quaint::{ast::*, visitor::{Visitor, Sqlite}};
    /// # fn main() -> Result<(), quaint::error::Error> {
    /// let query = Select::from_table("users").so_that("foo".like("%bar%"));
    /// let (sql, params) = Sqlite::build(query)?;
    ///
    /// assert_eq!("SELECT `users`.* FROM `users` WHERE `foo` LIKE ?", sql);
    ///
    /// assert_eq!(
    ///     vec![
    ///         Value::from("%bar%"),
    ///     ],
    ///     params
    /// );
    /// # Ok(())
    /// # }
    /// ```
    fn like<T>(self, pattern: T) -> Compare<'a>
    where
        T: Into<Expression<'a>>;

    /// Tests if the left side does not include the right side string.
    ///
    /// ```rust
    /// # use quaint::{ast::*, visitor::{Visitor, Sqlite}};
    /// # fn main() -> Result<(), quaint::error::Error> {
    /// let query = Select::from_table("users").so_that("foo".not_like("%bar%"));
    /// let (sql, params) = Sqlite::build(query)?;
    ///
    /// assert_eq!("SELECT `users`.* FROM `users` WHERE `foo` NOT LIKE ?", sql);
    ///
    /// assert_eq!(
    ///     vec![
    ///         Value::from("%bar%"),
    ///     ],
    ///     params
    /// );
    /// # Ok(())
    /// # }
    /// ```
    fn not_like<T>(self, pattern: T) -> Compare<'a>
    where
        T: Into<Expression<'a>>;

    /// Tests if the left side is `NULL`.
    ///
    /// ```rust
    /// # use quaint::{ast::*, visitor::{Visitor, Sqlite}};
    /// # fn main() -> Result<(), quaint::error::Error> {
    /// let query = Select::from_table("users").so_that("foo".is_null());
    /// let (sql, _) = Sqlite::build(query)?;
    ///
    /// assert_eq!("SELECT `users`.* FROM `users` WHERE `foo` IS NULL", sql);
    /// # Ok(())
    /// # }
    /// ```
    #[allow(clippy::wrong_self_convention)]
    fn is_null(self) -> Compare<'a>;

    /// Tests if the left side is not `NULL`.
    ///
    /// ```rust
    /// # use quaint::{ast::*, visitor::{Visitor, Sqlite}};
    /// # fn main() -> Result<(), quaint::error::Error> {
    /// let query = Select::from_table("users").so_that("foo".is_not_null());
    /// let (sql, _) = Sqlite::build(query)?;
    ///
    /// assert_eq!("SELECT `users`.* FROM `users` WHERE `foo` IS NOT NULL", sql);
    /// # Ok(())
    /// # }
    /// ```
    #[allow(clippy::wrong_self_convention)]
    fn is_not_null(self) -> Compare<'a>;

    /// Tests if the value is between two given values.
    ///
    /// ```rust
    /// # use quaint::{ast::*, visitor::{Visitor, Sqlite}};
    /// # fn main() -> Result<(), quaint::error::Error> {
    /// let query = Select::from_table("users").so_that("foo".between(420, 666));
    /// let (sql, params) = Sqlite::build(query)?;
    ///
    /// assert_eq!("SELECT `users`.* FROM `users` WHERE `foo` BETWEEN ? AND ?", sql);
    ///
    /// assert_eq!(vec![
    ///     Value::from(420),
    ///     Value::from(666),
    /// ], params);
    /// # Ok(())
    /// # }
    /// ```
    fn between<T, V>(self, left: T, right: V) -> Compare<'a>
    where
        T: Into<Expression<'a>>,
        V: Into<Expression<'a>>;

    /// Tests if the value is not between two given values.
    ///
    /// ```rust
    /// # use quaint::{ast::*, visitor::{Visitor, Sqlite}};
    /// # fn main() -> Result<(), quaint::error::Error> {
    /// let query = Select::from_table("users").so_that("foo".not_between(420, 666));
    /// let (sql, params) = Sqlite::build(query)?;
    ///
    /// assert_eq!("SELECT `users`.* FROM `users` WHERE `foo` NOT BETWEEN ? AND ?", sql);
    ///
    /// assert_eq!(vec![
    ///     Value::from(420),
    ///     Value::from(666),
    /// ], params);
    /// # Ok(())
    /// # }
    /// ```
    fn not_between<T, V>(self, left: T, right: V) -> Compare<'a>
    where
        T: Into<Expression<'a>>,
        V: Into<Expression<'a>>;

    /// Tests if the JSON array contains a value.
    ///
    /// ```rust
    /// # use quaint::{ast::*, visitor::{Visitor, Mysql}};
    /// # fn main() -> Result<(), quaint::error::Error> {
    /// let query = Select::from_table("users")
    ///     .so_that("json".json_array_contains(serde_json::json!(1)));
    /// let (sql, params) = Mysql::build(query)?;
    ///
    /// assert_eq!("SELECT `users`.* FROM `users` WHERE JSON_CONTAINS(`json`, ?)", sql);
    ///
    /// assert_eq!(vec![Value::from(serde_json::json!(1))], params);
    /// # Ok(())
    /// # }
    /// ```
    #[cfg(any(feature = "postgresql", feature = "mysql"))]
    fn json_array_contains<T>(self, item: T) -> Compare<'a>
    where
        T: Into<Expression<'a>>;

    /// Tests if the JSON array does not contain a value.
    ///
    /// ```rust
    /// # use quaint::{ast::*, visitor::{Visitor, Mysql}};
    /// # fn main() -> Result<(), quaint::error::Error> {
    /// let query = Select::from_table("users")
    ///     .so_that("json".json_array_not_contains(serde_json::json!(1)));
    /// let (sql, params) = Mysql::build(query)?;
    ///
    /// assert_eq!("SELECT `users`.* FROM `users` WHERE JSON_CONTAINS(`json`, ?) = FALSE", sql);
    /// assert_eq!(vec![Value::from(serde_json::json!(1))], params);
    ///
    /// # Ok(())
    /// # }
    /// ```
    #[cfg(any(feature = "postgresql", feature = "mysql"))]
    fn json_array_not_contains<T>(self, item: T) -> Compare<'a>
    where
        T: Into<Expression<'a>>;

    /// Tests if the JSON array starts with a value.
    ///
    /// ```rust
    /// # use quaint::{ast::*, visitor::{Visitor, Mysql}};
    /// # fn main() -> Result<(), quaint::error::Error> {
    /// let query = Select::from_table("users")
    ///     .so_that("json".json_array_begins_with(serde_json::json!(1)));
    /// let (sql, params) = Mysql::build(query)?;
    ///
    /// assert_eq!(
    ///   "SELECT `users`.* FROM `users` WHERE \
    ///      (JSON_CONTAINS(JSON_EXTRACT(`json`, ?), ?) AND \
    ///      JSON_CONTAINS(?, JSON_EXTRACT(`json`, ?)))",
    ///   sql
    /// );
    /// assert_eq!(vec![
    ///     Value::from("$[0]"),
    ///     Value::from(serde_json::json!(1)),
    ///     Value::from(serde_json::json!(1)),
    ///     Value::from("$[0]"),
    /// ], params);
    ///
    /// # Ok(())
    /// # }
    /// ```
    #[cfg(any(feature = "postgresql", feature = "mysql"))]
    fn json_array_begins_with<T>(self, item: T) -> Compare<'a>
    where
        T: Into<Expression<'a>>;

    /// Tests if the JSON array does not start with a value.
    ///
    /// ```rust
    /// # use quaint::{ast::*, visitor::{Visitor, Mysql}};
    /// # fn main() -> Result<(), quaint::error::Error> {
    /// let query = Select::from_table("users")
    ///   .so_that("json".json_array_not_begins_with(serde_json::json!(1)));
    /// let (sql, params) = Mysql::build(query)?;
    ///
    /// assert_eq!(
    ///   "SELECT `users`.* FROM `users` WHERE \
    ///      (NOT JSON_CONTAINS(JSON_EXTRACT(`json`, ?), ?) OR \
    ///      NOT JSON_CONTAINS(?, JSON_EXTRACT(`json`, ?)))",
    ///   sql
    /// );
    /// assert_eq!(vec![
    ///     Value::from("$[0]"),
    ///     Value::from(serde_json::json!(1)),
    ///     Value::from(serde_json::json!(1)),
    ///     Value::from("$[0]"),
    /// ], params);
    ///
    /// # Ok(())
    /// # }
    /// ```
    #[cfg(any(feature = "postgresql", feature = "mysql"))]
    fn json_array_not_begins_with<T>(self, item: T) -> Compare<'a>
    where
        T: Into<Expression<'a>>;

    /// Tests if the JSON array ends with a value.
    ///
    /// ```rust
    /// # use quaint::{ast::*, visitor::{Visitor, Mysql}};
    /// # fn main() -> Result<(), quaint::error::Error> {
    /// let query = Select::from_table("users")
    ///     .so_that("json".json_array_ends_into(serde_json::json!(1)));
    /// let (sql, params) = Mysql::build(query)?;
    ///
    /// assert_eq!(
    ///   "SELECT `users`.* FROM `users` WHERE \
    ///      (JSON_CONTAINS(JSON_EXTRACT(`json`, CONCAT('$[', JSON_LENGTH(`json`) - 1, ']')), ?) AND \
    ///      JSON_CONTAINS(?, JSON_EXTRACT(`json`, CONCAT('$[', JSON_LENGTH(`json`) - 1, ']'))))",
    ///   sql
    /// );
    /// assert_eq!(vec![
    ///    Value::from(serde_json::json!(1)),
    ///    Value::from(serde_json::json!(1)),
    /// ], params);
    ///
    /// # Ok(())
    /// # }
    /// ```
    #[cfg(any(feature = "postgresql", feature = "mysql"))]
    fn json_array_ends_into<T>(self, item: T) -> Compare<'a>
    where
        T: Into<Expression<'a>>;

    /// Tests if the JSON array does not end with a value.
    ///
    /// ```rust
    /// # use quaint::{ast::*, visitor::{Visitor, Mysql}};
    /// # fn main() -> Result<(), quaint::error::Error> {
    /// let query = Select::from_table("users").so_that("json".json_array_not_ends_into(serde_json::json!(1)));
    /// let (sql, params) = Mysql::build(query)?;
    ///
    /// assert_eq!(
    ///   "SELECT `users`.* FROM `users` WHERE \
    ///      (NOT JSON_CONTAINS(JSON_EXTRACT(`json`, CONCAT('$[', JSON_LENGTH(`json`) - 1, ']')), ?) OR \
    ///      NOT JSON_CONTAINS(?, JSON_EXTRACT(`json`, CONCAT('$[', JSON_LENGTH(`json`) - 1, ']'))))",
    ///   sql
    /// );
    ///
    /// assert_eq!(vec![
    ///    Value::from(serde_json::json!(1)),
    ///    Value::from(serde_json::json!(1)),
    /// ], params);
    ///
    /// # Ok(())
    /// # }
    /// ```
    #[cfg(any(feature = "postgresql", feature = "mysql"))]
    fn json_array_not_ends_into<T>(self, item: T) -> Compare<'a>
    where
        T: Into<Expression<'a>>;

    /// Tests if the JSON value is of a certain type.
    ///
    /// ```rust
    /// # use quaint::{ast::*, visitor::{Visitor, Mysql}};
    /// # fn main() -> Result<(), quaint::error::Error> {
    /// let query = Select::from_table("users").so_that("json".json_type_equals(JsonType::Array));
    /// let (sql, params) = Mysql::build(query)?;
    ///
    /// assert_eq!("SELECT `users`.* FROM `users` WHERE (JSON_TYPE(`json`) = ?)", sql);
    ///
    /// assert_eq!(vec![Value::from("ARRAY")], params);
    /// # Ok(())
    /// # }
    /// ```
    #[cfg(any(feature = "postgresql", feature = "mysql"))]
    fn json_type_equals<T>(self, json_type: T) -> Compare<'a>
    where
        T: Into<JsonType<'a>>;

    /// Tests if the JSON value is not of a certain type.
    ///
    /// ```rust
    /// # use quaint::{ast::*, visitor::{Visitor, Mysql}};
    /// # fn main() -> Result<(), quaint::error::Error> {
    /// let query = Select::from_table("users").so_that("json".json_type_not_equals(JsonType::Array));
    /// let (sql, params) = Mysql::build(query)?;
    ///
    /// assert_eq!("SELECT `users`.* FROM `users` WHERE (JSON_TYPE(`json`) != ?)", sql);
    ///
    /// assert_eq!(vec![Value::from("ARRAY")], params);
    /// # Ok(())
    /// # }
    /// ```
    #[cfg(any(feature = "postgresql", feature = "mysql"))]
    fn json_type_not_equals<T>(self, json_type: T) -> Compare<'a>
    where
        T: Into<JsonType<'a>>;

    /// Tests if a full-text search matches a certain query. Use it in combination with the `text_search()` function
    ///
    /// ```rust
    /// # use quaint::{ast::*, visitor::{Visitor, Postgres}};
    /// # fn main() -> Result<(), quaint::error::Error> {
    /// let search: Expression = text_search(&[Column::from("name"), Column::from("ingredients")]).into();
    /// let query = Select::from_table("recipes").so_that(search.matches("chicken"));
    /// let (sql, params) = Postgres::build(query)?;
    ///
    /// assert_eq!(
    ///    "SELECT \"recipes\".* FROM \"recipes\" \
    ///     WHERE to_tsvector(concat_ws(' ', \"name\",\"ingredients\")) @@ to_tsquery($1)", sql
    /// );
    ///
    /// assert_eq!(params, vec![Value::from("chicken")]);
    ///
    /// # Ok(())    
    /// # }
    /// ```
    #[cfg(feature = "postgresql")]
    fn matches<T>(self, query: T) -> Compare<'a>
    where
        T: Into<Cow<'a, str>>;

    /// Tests if a full-text search does not match a certain query. Use it in combination with the `text_search()` function
    ///
    /// ```rust
    /// # use quaint::{ast::*, visitor::{Visitor, Postgres}};
    /// # fn main() -> Result<(), quaint::error::Error> {
    /// let search: Expression = text_search(&[Column::from("name"), Column::from("ingredients")]).into();
    /// let query = Select::from_table("recipes").so_that(search.not_matches("chicken"));
    /// let (sql, params) = Postgres::build(query)?;
    ///
    /// assert_eq!(
    ///    "SELECT \"recipes\".* FROM \"recipes\" \
    ///     WHERE (NOT to_tsvector(concat_ws(' ', \"name\",\"ingredients\")) @@ to_tsquery($1))", sql
    /// );
    ///
    /// assert_eq!(params, vec![Value::from("chicken")]);
    ///
    /// # Ok(())    
    /// # }
    /// ```
    #[cfg(feature = "postgresql")]
    fn not_matches<T>(self, query: T) -> Compare<'a>
    where
        T: Into<Cow<'a, str>>;

    /// Matches at least one elem of a list of values.
    ///
    /// ```rust
    /// # use quaint::{ast::*, col, visitor::{Visitor, Postgres}};
    /// # fn main() -> Result<(), quaint::error::Error> {
    /// let query = Select::from_table("users").so_that(col!("name").equals(col!("list").any()));
    /// let (sql, _) = Postgres::build(query)?;
    /// assert_eq!(r#"SELECT "users".* FROM "users" WHERE "name" = ANY("list")"#, sql);
    /// # Ok(())
    /// # }
    /// ```
    #[cfg(feature = "postgresql")]
    fn any(self) -> Compare<'a>;

    /// Matches all elem of a list of values.
    ///
    /// ```rust
    /// # use quaint::{ast::*, col, visitor::{Visitor, Postgres}};
    /// # fn main() -> Result<(), quaint::error::Error> {
    /// let query = Select::from_table("users").so_that(col!("name").equals(col!("list").all()));
    /// let (sql, _) = Postgres::build(query)?;
    /// assert_eq!(r#"SELECT "users".* FROM "users" WHERE "name" = ALL("list")"#, sql);
    /// # Ok(())
    /// # }
    /// ```
    #[cfg(feature = "postgresql")]
    fn all(self) -> Compare<'a>;

    /// Compares two expressions with a custom operator.
    ///
    /// ```rust
    /// # use quaint::{ast::*, visitor::{Visitor, Sqlite}};
    /// # fn main() -> Result<(), quaint::error::Error> {
    /// let query = Select::from_table("users").so_that("foo".compare_raw("ILIKE", "%bar%"));
    /// let (sql, params) = Sqlite::build(query)?;
    ///
    /// assert_eq!("SELECT `users`.* FROM `users` WHERE `foo` ILIKE ?", sql);
    ///
    /// assert_eq!(vec![
    ///     Value::from("%bar%"),
    /// ], params);
    ///
    /// # Ok(())
    /// # }
    /// ```
    fn compare_raw<T, V>(self, raw_comparator: T, right: V) -> Compare<'a>
    where
        T: Into<Cow<'a, str>>,
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
        T: Into<Expression<'a>>,
    {
        let col: Column<'a> = self.into();
        let val: Expression<'a> = col.into();
        val.like(pattern)
    }

    fn not_like<T>(self, pattern: T) -> Compare<'a>
    where
        T: Into<Expression<'a>>,
    {
        let col: Column<'a> = self.into();
        let val: Expression<'a> = col.into();
        val.not_like(pattern)
    }

    #[allow(clippy::wrong_self_convention)]
    fn is_null(self) -> Compare<'a> {
        let col: Column<'a> = self.into();
        let val: Expression<'a> = col.into();
        val.is_null()
    }

    #[allow(clippy::wrong_self_convention)]
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

    fn compare_raw<T, V>(self, raw_comparator: T, right: V) -> Compare<'a>
    where
        T: Into<Cow<'a, str>>,
        V: Into<Expression<'a>>,
    {
        let left: Column<'a> = self.into();
        let left: Expression<'a> = left.into();
        let right: Expression<'a> = right.into();

        left.compare_raw(raw_comparator.into(), right)
    }

    #[cfg(any(feature = "postgresql", feature = "mysql"))]
    fn json_array_contains<T>(self, item: T) -> Compare<'a>
    where
        T: Into<Expression<'a>>,
    {
        let col: Column<'a> = self.into();
        let val: Expression<'a> = col.into();

        val.json_array_contains(item)
    }

    #[cfg(any(feature = "postgresql", feature = "mysql"))]
    fn json_array_not_contains<T>(self, item: T) -> Compare<'a>
    where
        T: Into<Expression<'a>>,
    {
        let col: Column<'a> = self.into();
        let val: Expression<'a> = col.into();

        val.json_array_not_contains(item)
    }

    #[cfg(any(feature = "postgresql", feature = "mysql"))]
    fn json_array_begins_with<T>(self, item: T) -> Compare<'a>
    where
        T: Into<Expression<'a>>,
    {
        let col: Column<'a> = self.into();
        let val: Expression<'a> = col.into();

        val.json_array_begins_with(item)
    }

    #[cfg(any(feature = "postgresql", feature = "mysql"))]
    fn json_array_not_begins_with<T>(self, item: T) -> Compare<'a>
    where
        T: Into<Expression<'a>>,
    {
        let col: Column<'a> = self.into();
        let val: Expression<'a> = col.into();

        val.json_array_not_begins_with(item)
    }

    #[cfg(any(feature = "postgresql", feature = "mysql"))]
    fn json_array_ends_into<T>(self, item: T) -> Compare<'a>
    where
        T: Into<Expression<'a>>,
    {
        let col: Column<'a> = self.into();
        let val: Expression<'a> = col.into();

        val.json_array_ends_into(item)
    }

    #[cfg(any(feature = "postgresql", feature = "mysql"))]
    fn json_array_not_ends_into<T>(self, item: T) -> Compare<'a>
    where
        T: Into<Expression<'a>>,
    {
        let col: Column<'a> = self.into();
        let val: Expression<'a> = col.into();

        val.json_array_not_ends_into(item)
    }

    #[cfg(any(feature = "postgresql", feature = "mysql"))]
    fn json_type_equals<T>(self, json_type: T) -> Compare<'a>
    where
        T: Into<JsonType<'a>>,
    {
        let col: Column<'a> = self.into();
        let val: Expression<'a> = col.into();

        val.json_type_equals(json_type)
    }

    #[cfg(any(feature = "postgresql", feature = "mysql"))]
    fn json_type_not_equals<T>(self, json_type: T) -> Compare<'a>
    where
        T: Into<JsonType<'a>>,
    {
        let col: Column<'a> = self.into();
        let val: Expression<'a> = col.into();

        val.json_type_not_equals(json_type)
    }

    #[cfg(feature = "postgresql")]
    fn matches<T>(self, query: T) -> Compare<'a>
    where
        T: Into<Cow<'a, str>>,
    {
        let col: Column<'a> = self.into();
        let val: Expression<'a> = col.into();

        val.matches(query)
    }

    #[cfg(feature = "postgresql")]
    fn not_matches<T>(self, query: T) -> Compare<'a>
    where
        T: Into<Cow<'a, str>>,
    {
        let col: Column<'a> = self.into();
        let val: Expression<'a> = col.into();

        val.not_matches(query)
    }

    #[cfg(feature = "postgresql")]
    fn any(self) -> Compare<'a> {
        let col: Column<'a> = self.into();
        let val: Expression<'a> = col.into();

        val.any()
    }

    #[cfg(feature = "postgresql")]
    fn all(self) -> Compare<'a> {
        let col: Column<'a> = self.into();
        let val: Expression<'a> = col.into();

        val.all()
    }
}
