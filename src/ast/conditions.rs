use crate::ast::{Compare, Expression};

#[derive(Debug, PartialEq, Clone)]
pub enum ConditionTree {
    And(Box<Expression>, Box<Expression>),
    Or(Box<Expression>, Box<Expression>),
    Not(Box<Expression>),
    Single(Box<Expression>),
    NoCondition,
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

impl Into<ConditionTree> for Compare {
    fn into(self) -> ConditionTree {
        let expression: Expression = self.into();
        ConditionTree::single(expression)
    }
}

pub trait And {
    fn and<E>(self, other: E) -> ConditionTree
    where
        E: Into<Expression>;
}
