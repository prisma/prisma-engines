use crate::ast::{Compare, ConditionTree, Like};
use crate::visitor::{Destination, Visitor};

#[derive(Debug, PartialEq, Clone)]
pub enum Expression {
    ConditionTree(ConditionTree),
    Compare(Compare),
    Like(Box<Like>),
}

impl Destination for Expression {
    fn visit(&self, visitor: &mut Visitor) {
        match self {
            Expression::ConditionTree(ref tree) => visitor.visit_condition_tree(tree),
            Expression::Compare(ref compare) => visitor.visit_compare(compare),
            Expression::Like(ref like) => visitor.visit_like(like),
        }
    }
}
