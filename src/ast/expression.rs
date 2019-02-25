use crate::ast::{Compare, ConditionTree, DatabaseValue};

#[derive(Debug, PartialEq, Clone)]
pub enum Expression {
    ConditionTree(ConditionTree),
    Compare(Compare),
    Value(DatabaseValue),
}
