use crate::ast::*;

/// Tree structures and leaves for condition building.
#[derive(Debug, PartialEq, Clone)]
pub enum ConditionTree<'a> {
    /// `(left_expression AND right_expression)`
    And(Box<Expression<'a>>, Box<Expression<'a>>),
    /// `(left_expression OR right_expression)`
    Or(Box<Expression<'a>>, Box<Expression<'a>>),
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
    pub fn and<E, J>(left: E, right: J) -> ConditionTree<'a>
    where
        E: Into<Expression<'a>>,
        J: Into<Expression<'a>>,
    {
        ConditionTree::And(Box::new(left.into()), Box::new(right.into()))
    }

    /// An `OR` statement, is true when one side is true.
    #[inline]
    pub fn or<E, J>(left: E, right: J) -> ConditionTree<'a>
    where
        E: Into<Expression<'a>>,
        J: Into<Expression<'a>>,
    {
        ConditionTree::Or(Box::new(left.into()), Box::new(right.into()))
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
