use crate::ast::*;

#[cfg(feature = "sqlite")]
pub mod sqlite;

pub trait Visitor {
    fn visit(&mut self, query: &Query) {
        match query {
            Query::Select(ref select) => self.visit_select(select),
        }
    }

    fn visit_select(&mut self, select: &Select);
    fn visit_condition_tree(&mut self, tree: &ConditionTree);
    fn visit_compare(&mut self, compare: &Compare);
    fn visit_ordering(&mut self, ordering: &Ordering);
    fn visit_like(&mut self, like: &Like);
}

pub trait Destination {
    fn visit(&self, visitor: &mut Visitor);
}
