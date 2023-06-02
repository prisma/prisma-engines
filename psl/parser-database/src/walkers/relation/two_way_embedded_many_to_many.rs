use crate::{
    relations::{Relation, RelationAttributes},
    walkers::{ModelWalker, RelationFieldWalker, RelationWalker},
};

/// Describes an explicit m:n relation between two models. Both sides define
/// `fields` which must be a single array scalar field, and `references` that
/// should point to a single scalar field on the referenced model.
#[derive(Copy, Clone)]
pub struct TwoWayEmbeddedManyToManyRelationWalker<'db>(pub(super) RelationWalker<'db>);

impl<'db> TwoWayEmbeddedManyToManyRelationWalker<'db> {
    /// Gets the relation attributes from the AST.
    fn get(&self) -> &'db Relation {
        &self.0.db.relations[self.0.id]
    }

    /// The model which comes first in the alphabetical order.
    pub fn model_a(self) -> ModelWalker<'db> {
        self.0.db.walk(self.get().model_a)
    }

    /// The model which comes after model a in the alphabetical order.
    pub fn model_b(self) -> ModelWalker<'db> {
        self.0.db.walk(self.get().model_b)
    }

    /// The field that defines the relation in model a.
    pub fn field_a(self) -> RelationFieldWalker<'db> {
        let rel = self.get();
        match rel.attributes {
            RelationAttributes::TwoWayEmbeddedManyToMany { field_a, field_b: _ } => self.0.walk(field_a),
            _ => unreachable!(),
        }
    }

    /// The field that defines the relation in model b.
    pub fn field_b(self) -> RelationFieldWalker<'db> {
        let rel = self.get();
        match rel.attributes {
            RelationAttributes::TwoWayEmbeddedManyToMany { field_a: _, field_b } => self.0.walk(field_b),

            _ => unreachable!(),
        }
    }
}
