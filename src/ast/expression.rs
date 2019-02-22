use crate::ast::{And, Compare, ConditionTree, DatabaseValue, Like};

#[derive(Debug, PartialEq, Clone)]
pub enum Expression {
    ConditionTree(ConditionTree),
    Compare(Compare),
    Like(Box<Like>),
    Value(DatabaseValue),
}

impl Into<Expression> for Compare {
    fn into(self) -> Expression {
        Expression::Compare(self)
    }
}

impl And for Compare {
    fn and<E>(self, other: E) -> ConditionTree
    where
        E: Into<Expression>,
    {
        let left: Expression = self.into();
        let right: Expression = other.into();

        ConditionTree::and(left, right)
    }
}
