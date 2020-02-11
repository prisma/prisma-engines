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
    #[inline]
    pub fn and<E>(mut self, other: E) -> ConditionTree<'a>
    where
        E: Into<Expression<'a>>,
    {
        match self {
            Self::And(ref mut conditions) => {
                conditions.push(other.into());
                self
            }
            Self::Or(_) => Self::And(vec![Expression::from(self), other.into()]),
            Self::Not(_) => Self::And(vec![Expression::from(self), other.into()]),
            Self::Single(expr) => Self::And(vec![*expr, other.into()]),
            Self::NoCondition => self,
            Self::NegativeCondition => Self::And(vec![Expression::from(self), other.into()]),
        }
    }

    /// An `OR` statement, is true when one side is true.
    #[inline]
    pub fn or<E>(mut self, other: E) -> ConditionTree<'a>
    where
        E: Into<Expression<'a>>,
    {
        match self {
            Self::Or(ref mut conditions) => {
                conditions.push(other.into());
                self
            }
            Self::And(_) => Self::Or(vec![Expression::from(self), other.into()]),
            Self::Not(_) => Self::Or(vec![Expression::from(self), other.into()]),
            Self::Single(expr) => Self::Or(vec![*expr, other.into()]),
            Self::NoCondition => self,
            Self::NegativeCondition => Self::Or(vec![Expression::from(self), other.into()]),
        }
    }

    /// A `NOT` statement, is true when the expression is false.
    #[inline]
    pub fn not<E>(left: E) -> ConditionTree<'a>
    where
        E: Into<Expression<'a>>,
    {
        ConditionTree::Not(Box::new(left.into()))
    }

    /// A single leaf, is true when the expression is true.
    #[inline]
    pub fn single<E>(left: E) -> ConditionTree<'a>
    where
        E: Into<Expression<'a>>,
    {
        ConditionTree::Single(Box::new(left.into()))
    }

    /// Inverts the entire condition tree if condition is met.
    #[inline]
    pub fn invert_if(self, invert: bool) -> ConditionTree<'a> {
        if invert {
            self.not()
        } else {
            self
        }
    }
}

impl<'a> Default for ConditionTree<'a> {
    #[inline]
    fn default() -> Self {
        ConditionTree::NoCondition
    }
}

impl<'a> From<ConditionTree<'a>> for Expression<'a> {
    #[inline]
    fn from(ct: ConditionTree<'a>) -> Self {
        Expression::ConditionTree(ct)
    }
}

impl<'a> From<Select<'a>> for ConditionTree<'a> {
    #[inline]
    fn from(sel: Select<'a>) -> Self {
        ConditionTree::single(Expression::Value(Box::new(sel.into())))
    }
}
