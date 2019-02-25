use crate::ast::{Conjuctive, Expression};

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
    pub fn and<E, J>(left: E, right: J) -> ConditionTree
    where
        E: Into<Expression>,
        J: Into<Expression>,
    {
        ConditionTree::And(Box::new(left.into()), Box::new(right.into()))
    }

    pub fn or<E, J>(left: E, right: J) -> ConditionTree
    where
        E: Into<Expression>,
        J: Into<Expression>,
    {
        ConditionTree::Or(Box::new(left.into()), Box::new(right.into()))
    }

    pub fn not<E>(left: E) -> ConditionTree
    where
        E: Into<Expression>,
    {
        ConditionTree::Not(Box::new(left.into()))
    }

    pub fn single<E>(left: E) -> ConditionTree
    where
        E: Into<Expression>,
    {
        ConditionTree::Single(Box::new(left.into()))
    }
}

impl Default for ConditionTree {
    fn default() -> Self {
        ConditionTree::NoCondition
    }
}

impl Into<Expression> for ConditionTree {
    fn into(self) -> Expression {
        Expression::ConditionTree(self)
    }
}

impl Conjuctive for ConditionTree {
    fn and<E>(self, other: E) -> ConditionTree
    where
        E: Into<Expression>,
    {
        let left: Expression = self.into();
        left.and(other)
    }

    fn or<E>(self, other: E) -> ConditionTree
    where
        E: Into<Expression>,
    {
        let left: Expression = self.into();
        left.or(other)
    }

    fn not(self) -> ConditionTree {
        let exp: Expression = self.into();
        exp.not()
    }
}

impl Conjuctive for Expression {
    fn and<E>(self, other: E) -> ConditionTree
    where
        E: Into<Expression>,
    {
        let right: Expression = other.into();
        ConditionTree::and(self, right)
    }

    fn or<E>(self, other: E) -> ConditionTree
    where
        E: Into<Expression>,
    {
        let right: Expression = other.into();
        ConditionTree::or(self, right)
    }

    fn not(self) -> ConditionTree {
        ConditionTree::not(self)
    }
}
