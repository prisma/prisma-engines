use std::{fmt, rc::Rc};

use itertools::Itertools;

use crate::transform::ast_to_dml::db::walkers::CompleteInlineRelationWalker;

/// A linked list structure for visited relation paths.
#[derive(Clone)]
pub(super) struct VisitedRelation<'ast, 'db> {
    previous: Option<Rc<VisitedRelation<'ast, 'db>>>,
    relation: CompleteInlineRelationWalker<'ast, 'db>,
}

impl<'ast, 'db> VisitedRelation<'ast, 'db> {
    /// Create a new root node, starting a new relation path.
    pub(super) fn root(relation: CompleteInlineRelationWalker<'ast, 'db>) -> Self {
        Self {
            previous: None,
            relation,
        }
    }

    /// Links a relation to the current path.
    pub(super) fn link_next(self: &Rc<Self>, relation: CompleteInlineRelationWalker<'ast, 'db>) -> Self {
        Self {
            previous: Some(self.clone()),
            relation,
        }
    }

    /// Converts the list into an iterator.
    pub(super) fn iter(&self) -> VisitedRelationIter<'ast, 'db> {
        let mut traversed = vec![self.relation];
        let mut this = self;

        while let Some(next) = this.previous.as_ref() {
            traversed.push(next.relation);
            this = next;
        }

        VisitedRelationIter { traversed }
    }
}

impl<'ast, 'db> fmt::Display for VisitedRelation<'ast, 'db> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut traversed = self.iter().map(|relation| {
            format!(
                "{}.{}",
                relation.referencing_model().name(),
                relation.referencing_field().ast_field().name()
            )
        });

        write!(f, "{}", traversed.join(" → "))
    }
}

pub(super) struct VisitedRelationIter<'ast, 'db> {
    traversed: Vec<CompleteInlineRelationWalker<'ast, 'db>>,
}

impl<'ast, 'db> Iterator for VisitedRelationIter<'ast, 'db> {
    type Item = CompleteInlineRelationWalker<'ast, 'db>;

    fn next(&mut self) -> Option<Self::Item> {
        self.traversed.pop()
    }
}
