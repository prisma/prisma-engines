use crate::ast::*;
use either::Either;
use query::SelectQuery;
use std::borrow::Cow;

/// An expression that can be positioned in a query. Can be a single value or a
/// statement that is evaluated into a value.
#[derive(Debug, Clone, PartialEq)]
pub struct Expression<'a> {
    pub(crate) kind: ExpressionKind<'a>,
    pub(crate) alias: Option<Cow<'a, str>>,
}

impl<'a> Expression<'a> {
    /// The type of the expression, dictates how it's implemented in the query.
    pub fn kind(&self) -> &ExpressionKind<'a> {
        &self.kind
    }

    /// The name alias of the expression, how it can referred in the query.
    pub fn alias(&self) -> Option<&str> {
        self.alias.as_ref().map(|s| s.as_ref())
    }

    pub(crate) fn row(row: Row<'a>) -> Self {
        Self {
            kind: ExpressionKind::Row(row),
            alias: None,
        }
    }

    pub(crate) fn union(union: Union<'a>) -> Self {
        Self::selection(SelectQuery::Union(Box::new(union)))
    }

    pub(crate) fn selection(selection: SelectQuery<'a>) -> Self {
        Self {
            kind: ExpressionKind::Selection(selection),
            alias: None,
        }
    }

    #[cfg(feature = "json-1")]
    pub(crate) fn is_json_value(&self) -> bool {
        match &self.kind {
            ExpressionKind::Parameterized(Value::Json(_)) => true,
            ExpressionKind::Value(expr) => expr.is_json_value(),
            _ => false,
        }
    }

    pub(crate) fn is_asterisk(&self) -> bool {
        matches!(self.kind, ExpressionKind::Asterisk(_))
    }

    pub(crate) fn is_row(&self) -> bool {
        matches!(self.kind, ExpressionKind::Row(_))
    }

    pub(crate) fn into_row(self) -> Option<Row<'a>> {
        match self.kind {
            ExpressionKind::Row(row) => Some(row),
            _ => None,
        }
    }

    pub(crate) fn into_selection(self) -> Option<SelectQuery<'a>> {
        match self.kind {
            ExpressionKind::Selection(selection) => Some(selection),
            _ => None,
        }
    }

    pub(crate) fn into_column(self) -> Option<Column<'a>> {
        match self.kind {
            ExpressionKind::Column(column) => Some(*column),
            _ => None,
        }
    }

    pub(crate) fn is_selection(&self) -> bool {
        matches!(self.kind, ExpressionKind::Selection(_))
    }

    pub(crate) fn is_column(&self) -> bool {
        matches!(self.kind, ExpressionKind::Column(_))
    }

    /// Finds all comparisons between a tuple and a selection. If returning some
    /// CTEs, they should be handled in the calling layer.
    pub(crate) fn convert_tuple_selects_to_ctes(
        self,
        level: &mut usize,
    ) -> (Self, Option<Vec<CommonTableExpression<'a>>>) {
        match self.kind {
            ExpressionKind::Selection(SelectQuery::Select(select)) => {
                let select = select.convert_tuple_selects_to_ctes(level);

                let expr = Expression {
                    kind: ExpressionKind::Selection(SelectQuery::Select(Box::new(select))),
                    alias: self.alias,
                };

                (expr, None)
            }
            ExpressionKind::Selection(SelectQuery::Union(union)) => {
                let union = union.convert_tuple_selects_into_ctes(level);

                let expr = Expression {
                    kind: ExpressionKind::Selection(SelectQuery::Union(Box::new(union))),
                    alias: self.alias,
                };

                (expr, None)
            }
            ExpressionKind::Compare(compare) => match compare.convert_tuple_select_to_cte(level) {
                // Conversion happened
                Either::Left((comp, cte)) => {
                    let expr = Expression {
                        kind: ExpressionKind::Compare(comp),
                        alias: self.alias,
                    };

                    (expr, Some(vec![cte]))
                }
                // No conversion
                Either::Right(compare) => {
                    let expr = Expression {
                        kind: ExpressionKind::Compare(compare),
                        alias: self.alias,
                    };

                    (expr, None)
                }
            },
            ExpressionKind::ConditionTree(tree) => {
                let (tree, ctes) = tree.convert_tuple_selects_to_ctes(level);

                let expr = Expression {
                    kind: ExpressionKind::ConditionTree(tree),
                    alias: self.alias,
                };

                (expr, ctes)
            }
            _ => (self, None),
        }
    }
}

/// An expression we can compare and use in database queries.
#[derive(Debug, Clone, PartialEq)]
pub enum ExpressionKind<'a> {
    /// Anything that we must parameterize before querying
    Parameterized(Value<'a>),
    /// A user-provided value we do not parameterize.
    RawValue(Raw<'a>),
    /// A database column
    Column(Box<Column<'a>>),
    /// Data in a row form, e.g. (1, 2, 3)
    Row(Row<'a>),
    /// A nested `SELECT` or `SELECT .. UNION` statement
    Selection(SelectQuery<'a>),
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

expression!(Row, Row);
expression!(Function, Function);

impl<'a> From<Raw<'a>> for Expression<'a> {
    fn from(r: Raw<'a>) -> Self {
        Expression {
            kind: ExpressionKind::RawValue(r),
            alias: None,
        }
    }
}

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

impl<'a> From<ExpressionKind<'a>> for Expression<'a> {
    fn from(kind: ExpressionKind<'a>) -> Self {
        Self { kind, alias: None }
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

    fn compare_raw<T, V>(self, raw_comparator: T, right: V) -> Compare<'a>
    where
        T: Into<Cow<'a, str>>,
        V: Into<Expression<'a>>,
    {
        Compare::Raw(Box::new(self), raw_comparator.into(), Box::new(right.into()))
    }
}
