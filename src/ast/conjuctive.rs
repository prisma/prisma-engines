use crate::ast::{ConditionTree, Expression};

pub trait Conjuctive {
    fn and<E>(self, other: E) -> ConditionTree
    where
        E: Into<Expression>;

    fn or<E>(self, other: E) -> ConditionTree
    where
        E: Into<Expression>;

    fn not(self) -> ConditionTree;
}
