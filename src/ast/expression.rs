use crate::ast::*;

/// A database expression.
#[derive(Debug, PartialEq, Clone)]
pub enum Expression<'a> {
    /// A tree of expressions to evaluate from the deepest value to up
    ConditionTree(ConditionTree<'a>),
    /// A comparison expression
    Compare(Compare<'a>),
    /// A single value, column, row or a nested select
    Value(Box<DatabaseValue<'a>>),
}

impl<'a> From<Select<'a>> for Expression<'a> {
    fn from(sel: Select<'a>) -> Expression<'a> {
        Expression::Value(Box::new(DatabaseValue::from(sel)))
    }
}
