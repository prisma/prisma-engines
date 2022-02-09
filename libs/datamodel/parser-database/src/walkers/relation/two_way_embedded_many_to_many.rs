use crate::{
    relations::{Relation, RelationAttributes},
    walkers::{ModelWalker, RelationFieldWalker},
};

use super::RelationWalker;

/// Describes an implicit m:n relation between two models. Neither side defines fields, attributes
/// or referential actions, which are all inferred by Prisma.
#[derive(Copy, Clone)]
pub struct TwoWayEmbeddedManyToManyRelationWalker<'db>(pub(super) RelationWalker<'db>);

impl<'ast, 'db> TwoWayEmbeddedManyToManyRelationWalker<'db> {
    /// Gets the relation attributes from the AST.
    fn get(&self) -> &'db Relation {
        &self.0.db.relations[self.0.id]
    }

    /// The model which comes first in the alphabetical order.
    pub fn model_a(self) -> ModelWalker<'db> {
        self.0.db.walk_model(self.get().model_a)
    }

    /// The model which comes after model a in the alphabetical order.
    pub fn model_b(self) -> ModelWalker<'db> {
        self.0.db.walk_model(self.get().model_b)
    }

    /// The field that defines the relation in model a.
    pub fn field_a(self) -> RelationFieldWalker<'db> {
        match self.get().attributes {
            RelationAttributes::TwoWayEmbeddedManyToMany { field_a, field_b: _ } => {
                self.model_a().relation_field(field_a)
            }
            _ => unreachable!(),
        }
    }

    /// The field that defines the relation in model b.
    pub fn field_b(self) -> RelationFieldWalker<'db> {
        match self.get().attributes {
            RelationAttributes::TwoWayEmbeddedManyToMany { field_a: _, field_b } => {
                self.model_b().relation_field(field_b)
            }
            _ => unreachable!(),
        }
    }
}
