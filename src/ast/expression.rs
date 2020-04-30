use crate::ast::*;
use std::borrow::Cow;

#[derive(Debug, Clone, PartialEq)]
pub struct Expression<'a> {
    pub(crate) kind: ExpressionKind<'a>,
    pub(crate) alias: Option<Cow<'a, str>>,
}

impl<'a> Expression<'a> {
    #[cfg(feature = "json-1")]
    pub(crate) fn is_json_value(&self) -> bool {
        match &self.kind {
            ExpressionKind::Parameterized(Value::Json(_)) => true,
            ExpressionKind::Value(expr) => expr.is_json_value(),
            _ => false,
        }
    }
}

/// An expression we can compare and use in database queries.
#[derive(Debug, Clone, PartialEq)]
pub enum ExpressionKind<'a> {
    /// Anything that we must parameterize before querying
    Parameterized(Value<'a>),
    /// A database column
    Column(Box<Column<'a>>),
    /// Data in a row form, e.g. (1, 2, 3)
    Row(Row<'a>),
    /// A nested `SELECT` statement
    Select(Box<Select<'a>>),
    /// A database function call
    Function(Function<'a>),
    /// A qualified asterisk to a table
    Asterisk(Option<Box<Table<'a>>>),
    /// An operation: sum, sub, mul or div.
    Op(Box<SqlOp<'a>>),
    /// A `VALUES` statement
    Values(Box<Values<'a>>),
    /// A tree of expressions to evaluate from the deepest value to up
    ConditionTree(ConditionTree<'a>),
    /// A comparison expression
    Compare(Compare<'a>),
    /// A single value, column, row or a nested select
    Value(Box<Expression<'a>>),
}

/// A quick alias to create an asterisk to a table.
pub fn asterisk() -> Expression<'static> {
    Expression {
        kind: ExpressionKind::Asterisk(None),
        alias: None,
    }
}

#[macro_export]
/// Marks a given string as a value. Useful when using a value in calculations,
/// e.g.
///
/// ``` rust
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
/// ```
macro_rules! val {
    ($val:expr) => {
        Expression::from($val)
    };
}

macro_rules! expression {
    ($kind:ident,$paramkind:ident) => {
        impl<'a> From<$kind<'a>> for Expression<'a> {
            fn from(that: $kind<'a>) -> Self {
                Expression {
                    kind: ExpressionKind::$paramkind(that),
                    alias: None,
                }
            }
        }
    };
}

expression!(Row, Row);
expression!(Function, Function);

impl<'a> From<Values<'a>> for Expression<'a> {
    fn from(p: Values<'a>) -> Self {
        Expression {
            kind: ExpressionKind::Values(Box::new(p)),
            alias: None,
        }
    }
}

impl<'a> From<SqlOp<'a>> for Expression<'a> {
    fn from(p: SqlOp<'a>) -> Self {
        Expression {
            kind: ExpressionKind::Op(Box::new(p)),
            alias: None,
        }
    }
}

impl<'a, T> From<T> for Expression<'a>
where
    T: Into<Value<'a>>,
{
    fn from(p: T) -> Self {
        Expression {
            kind: ExpressionKind::Parameterized(p.into()),
            alias: None,
        }
    }
}

impl<'a, T> From<Vec<T>> for Expression<'a>
where
    T: Into<Expression<'a>>,
{
    fn from(v: Vec<T>) -> Self {
        let row: Row<'a> = v.into();
        row.into()
    }
}

impl<'a> Aliasable<'a> for Expression<'a> {
    type Target = Expression<'a>;

    fn alias<T>(mut self, alias: T) -> Self::Target
    where
        T: Into<Cow<'a, str>>,
    {
        self.alias = Some(alias.into());
        self
    }
}

impl<'a> Comparable<'a> for Expression<'a> {
    fn equals<T>(self, comparison: T) -> Compare<'a>
    where
        T: Into<Expression<'a>>,
    {
        Compare::Equals(Box::new(self), Box::new(comparison.into()))
    }

    fn not_equals<T>(self, comparison: T) -> Compare<'a>
    where
        T: Into<Expression<'a>>,
    {
        Compare::NotEquals(Box::new(self), Box::new(comparison.into()))
    }

    fn less_than<T>(self, comparison: T) -> Compare<'a>
    where
        T: Into<Expression<'a>>,
    {
        Compare::LessThan(Box::new(self), Box::new(comparison.into()))
    }

    fn less_than_or_equals<T>(self, comparison: T) -> Compare<'a>
    where
        T: Into<Expression<'a>>,
    {
        Compare::LessThanOrEquals(Box::new(self), Box::new(comparison.into()))
    }

    fn greater_than<T>(self, comparison: T) -> Compare<'a>
    where
        T: Into<Expression<'a>>,
    {
        Compare::GreaterThan(Box::new(self), Box::new(comparison.into()))
    }

    fn greater_than_or_equals<T>(self, comparison: T) -> Compare<'a>
    where
        T: Into<Expression<'a>>,
    {
        Compare::GreaterThanOrEquals(Box::new(self), Box::new(comparison.into()))
    }

    fn in_selection<T>(self, selection: T) -> Compare<'a>
    where
        T: Into<Expression<'a>>,
    {
        Compare::In(Box::new(self), Box::new(selection.into()))
    }

    fn not_in_selection<T>(self, selection: T) -> Compare<'a>
    where
        T: Into<Expression<'a>>,
    {
        Compare::NotIn(Box::new(self), Box::new(selection.into()))
    }

    fn like<T>(self, pattern: T) -> Compare<'a>
    where
        T: Into<Cow<'a, str>>,
    {
        Compare::Like(Box::new(self), pattern.into())
    }

    fn not_like<T>(self, pattern: T) -> Compare<'a>
    where
        T: Into<Cow<'a, str>>,
    {
        Compare::NotLike(Box::new(self), pattern.into())
    }

    fn begins_with<T>(self, pattern: T) -> Compare<'a>
    where
        T: Into<Cow<'a, str>>,
    {
        Compare::BeginsWith(Box::new(self), pattern.into())
    }

    fn not_begins_with<T>(self, pattern: T) -> Compare<'a>
    where
        T: Into<Cow<'a, str>>,
    {
        Compare::NotBeginsWith(Box::new(self), pattern.into())
    }

    fn ends_into<T>(self, pattern: T) -> Compare<'a>
    where
        T: Into<Cow<'a, str>>,
    {
        Compare::EndsInto(Box::new(self), pattern.into())
    }

    fn not_ends_into<T>(self, pattern: T) -> Compare<'a>
    where
        T: Into<Cow<'a, str>>,
    {
        Compare::NotEndsInto(Box::new(self), pattern.into())
    }

    fn is_null(self) -> Compare<'a> {
        Compare::Null(Box::new(self))
    }

    fn is_not_null(self) -> Compare<'a> {
        Compare::NotNull(Box::new(self))
    }

    fn between<T, V>(self, left: T, right: V) -> Compare<'a>
    where
        T: Into<Expression<'a>>,
        V: Into<Expression<'a>>,
    {
        Compare::Between(Box::new(self), Box::new(left.into()), Box::new(right.into()))
    }

    fn not_between<T, V>(self, left: T, right: V) -> Compare<'a>
    where
        T: Into<Expression<'a>>,
        V: Into<Expression<'a>>,
    {
        Compare::NotBetween(Box::new(self), Box::new(left.into()), Box::new(right.into()))
    }
}
