use crate::ast::Expression;

#[derive(Debug, PartialEq, Clone)]
pub enum ConditionTree {
    And(Box<Expression>, Box<Expression>),
    Or(Box<Expression>, Box<Expression>),
    Not(Box<Expression>),
    Single(Box<Expression>),
    NoCondition,
    NegativeCondition,
}

impl Default for ConditionTree {
    fn default() -> Self {
        ConditionTree::NoCondition
    }
}
