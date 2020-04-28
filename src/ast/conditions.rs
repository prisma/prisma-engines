use crate::ast::*;

/// Tree structures and leaves for condition building.
#[derive(Debug, PartialEq, Clone)]
pub enum ConditionTree<'a> {
    /// `(left_expression AND right_expression)`
    And(Vec<Expression<'a>>),
    /// `(left_expression OR right_expression)`
    Or(Vec<Expression<'a>>),
    /// `(NOT expression)`
    Not(Box<Expression<'a>>),
    /// A single expression leaf
    Single(Box<Expression<'a>>),
    /// A leaf that does nothing to the condition, `1=1`
    NoCondition,
    /// A leaf that cancels the condition, `1=0`
    NegativeCondition,
}

impl<'a> ConditionTree<'a> {
    /// An `AND` statement, is true when both sides are true.
    pub fn and<E>(mut self, other: E) -> ConditionTree<'a>
    where
        E: Into<Expression<'a>>,
    {
        match self {
            Self::And(ref mut conditions) => {
                conditions.push(other.into());
                self
            }
            Self::Single(expr) => Self::And(vec![*expr, other.into()]),
            _ => Self::And(vec![Expression::from(self), other.into()]),
        }
    }

    /// An `OR` statement, is true when one side is true.
    pub fn or<E>(mut self, other: E) -> ConditionTree<'a>
    where
        E: Into<Expression<'a>>,
    {
        match self {
            Self::Or(ref mut conditions) => {
                conditions.push(other.into());
                self
            }
            Self::Single(expr) => Self::Or(vec![*expr, other.into()]),
            _ => Self::Or(vec![Expression::from(self), other.into()]),
        }
    }

    /// A `NOT` statement, is true when the expression is false.
    pub fn not<E>(left: E) -> ConditionTree<'a>
    where
        E: Into<Expression<'a>>,
    {
        ConditionTree::Not(Box::new(left.into()))
    }

    /// A single leaf, is true when the expression is true.
    pub fn single<E>(left: E) -> ConditionTree<'a>
    where
        E: Into<Expression<'a>>,
    {
        ConditionTree::Single(Box::new(left.into()))
    }

    /// Inverts the entire condition tree if condition is met.
    pub fn invert_if(self, invert: bool) -> ConditionTree<'a> {
        if invert {
            self.not()
        } else {
            self
        }
    }
}

impl<'a> Default for ConditionTree<'a> {
    fn default() -> Self {
        ConditionTree::NoCondition
    }
}

impl<'a> From<ConditionTree<'a>> for Expression<'a> {
    fn from(ct: ConditionTree<'a>) -> Self {
        Expression {
            kind: ExpressionKind::ConditionTree(ct),
            alias: None,
        }
    }
}

impl<'a> From<Select<'a>> for ConditionTree<'a> {
    fn from(sel: Select<'a>) -> Self {
        let exp = Expression {
            kind: ExpressionKind::Value(Box::new(sel.into())),
            alias: None,
        };

        ConditionTree::single(exp)
    }
}
