use crate::ast::*;

/// Tree structures and leaves for condition building.
#[derive(Debug, PartialEq, Clone)]
pub enum ConditionTree {
    /// `(left_expression AND right_expression)`
    And(Box<Expression>, Box<Expression>),
    /// `(left_expression OR right_expression)`
    Or(Box<Expression>, Box<Expression>),
    /// `(NOT expression)`
    Not(Box<Expression>),
    /// A single expression leaf
    Single(Box<Expression>),
    /// A leaf that does nothing to the condition, `1=1`
    NoCondition,
    /// A leaf that cancels the condition, `1=0`
    NegativeCondition,
}

impl ConditionTree {
    /// An `AND` statement, is true when both sides are true.
    #[inline]
    pub fn and<E, J>(left: E, right: J) -> ConditionTree
    where
        E: Into<Expression>,
        J: Into<Expression>,
    {
        ConditionTree::And(Box::new(left.into()), Box::new(right.into()))
    }

    /// An `OR` statement, is true when one side is true.
    #[inline]
    pub fn or<E, J>(left: E, right: J) -> ConditionTree
    where
        E: Into<Expression>,
        J: Into<Expression>,
    {
        ConditionTree::Or(Box::new(left.into()), Box::new(right.into()))
    }

    /// A `NOT` statement, is true when the expression is false.
    #[inline]
    pub fn not<E>(left: E) -> ConditionTree
    where
        E: Into<Expression>,
    {
        ConditionTree::Not(Box::new(left.into()))
    }

    /// A single leaf, is true when the expression is true.
    #[inline]
    pub fn single<E>(left: E) -> ConditionTree
    where
        E: Into<Expression>,
    {
        ConditionTree::Single(Box::new(left.into()))
    }

    /// Inverts the entire condition tree if condition is met.
    #[inline]
    pub fn invert_if(self, invert: bool) -> ConditionTree {
        if invert {
            self.not()
        } else {
            self
        }
    }
}

impl Default for ConditionTree {
    #[inline]
    fn default() -> Self {
        ConditionTree::NoCondition
    }
}

impl Into<Expression> for ConditionTree {
    #[inline]
    fn into(self) -> Expression {
        Expression::ConditionTree(self)
    }
}

impl From<Select> for ConditionTree {
    #[inline]
    fn from(sel: Select) -> ConditionTree {
        ConditionTree::single(Expression::Value(Box::new(sel.into())))
    }
}
