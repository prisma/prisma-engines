use itertools::Itertools;
use parser_database::walkers::CompleteInlineRelationWalker;
use std::{fmt, rc::Rc};

/// A linked list structure for visited relation paths.
#[derive(Clone)]
pub(super) struct VisitedRelation<'db> {
    previous: Option<Rc<VisitedRelation<'db>>>,
    relation: CompleteInlineRelationWalker<'db>,
}

impl<'db> VisitedRelation<'db> {
    /// Create a new root node, starting a new relation path.
    pub(super) fn root(relation: CompleteInlineRelationWalker<'db>) -> Self {
        Self {
            previous: None,
            relation,
        }
    }

    /// Links a relation to the current path.
    pub(super) fn link_next(self: &Rc<Self>, relation: CompleteInlineRelationWalker<'db>) -> Self {
        Self {
            previous: Some(self.clone()),
            relation,
        }
    }

    /// Converts the list into an iterator.
    pub(super) fn iter(&self) -> VisitedRelationIter<'db> {
        let mut traversed = vec![self.relation];
        let mut this = self;

        while let Some(next) = this.previous.as_ref() {
            traversed.push(next.relation);
            this = next;
        }

        VisitedRelationIter { traversed }
    }
}

impl<'db> fmt::Display for VisitedRelation<'db> {
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

pub(super) struct VisitedRelationIter<'db> {
    traversed: Vec<CompleteInlineRelationWalker<'db>>,
}

impl<'db> Iterator for VisitedRelationIter<'db> {
    type Item = CompleteInlineRelationWalker<'db>;

    fn next(&mut self) -> Option<Self::Item> {
        self.traversed.pop()
    }
}
