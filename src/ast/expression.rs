use crate::ast::{Compare, ConditionTree, DatabaseValue, Like};

#[derive(Debug, PartialEq, Clone)]
pub enum Expression {
    ConditionTree(ConditionTree),
    Compare(Compare),
    Like(Box<Like>),
    Value(DatabaseValue),
}
