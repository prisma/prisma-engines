use crate::{
    relations::{ManyToManyRelationId, Relation, RelationAttributes},
    walkers::{ModelWalker, RelationFieldWalker, RelationName, Walker},
};

/// Describes an implicit m:n relation between two models. Neither side defines fields, attributes
/// or referential actions, which are all inferred by Prisma.
pub type ImplicitManyToManyRelationWalker<'db> = Walker<'db, ManyToManyRelationId>;

impl<'db> ImplicitManyToManyRelationWalker<'db> {
    /// Gets the relation attributes from the AST.
    fn get(&self) -> &'db Relation {
        &self.db.relations[self.id.0]
    }

    /// The model which comes first in the alphabetical order.
    pub fn model_a(self) -> ModelWalker<'db> {
        self.db.walk(self.get().model_a)
    }

    /// The model which comes after model a in the alphabetical order.
    pub fn model_b(self) -> ModelWalker<'db> {
        self.db.walk(self.get().model_b)
    }

    /// The field that defines the relation in model a.
    pub fn field_a(self) -> RelationFieldWalker<'db> {
        match self.get().attributes {
            RelationAttributes::ImplicitManyToMany { field_a, field_b: _ } => self.model_a().relation_field(field_a),
            _ => unreachable!(),
        }
    }

    /// The field that defines the relation in model b.
    pub fn field_b(self) -> RelationFieldWalker<'db> {
        match self.get().attributes {
            RelationAttributes::ImplicitManyToMany { field_a: _, field_b } => self.model_b().relation_field(field_b),
            _ => unreachable!(),
        }
    }

    /// The name of the relation.
    pub fn relation_name(self) -> RelationName<'db> {
        self.field_a().relation_name()
    }
}
