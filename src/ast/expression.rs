use crate::ast::*;

/// A database expression.
#[derive(Debug, PartialEq, Clone)]
pub enum Expression {
    /// A tree of expressions to evaluate from the deepest value to up
    ConditionTree(ConditionTree),
    /// A comparison expression
    Compare(Compare),
    /// A single value, column, row or a nested select
    Value(DatabaseValue),
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

impl From<Select> for Expression {
    fn from(sel: Select) -> Expression {
        let dv: DatabaseValue = sel.into();
        Expression::Value(dv)
    }
}
