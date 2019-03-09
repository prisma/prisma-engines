use crate::ast::*;

/// A database expression.
#[derive(Debug, PartialEq, Clone)]
pub enum Expression {
    /// A tree of expressions to evaluate from the deepest value to up
    ConditionTree(ConditionTree),
    /// A comparison expression
    Compare(Compare),
    /// A single value, column, row or a nested select
    Value(Box<DatabaseValue>),
}

impl From<Select> for Expression {
    #[inline]
    fn from(sel: Select) -> Expression {
        let dv: DatabaseValue = sel.into();
        Expression::Value(Box::new(dv))
    }
}
