use std::fmt::Display;

use crate::{
    relations::{ManyToManyRelationId, Relation, RelationAttributes},
    walkers::{ModelWalker, RelationFieldWalker, RelationName, RelationWalker, Walker},
};

/// Describes an implicit m:n relation between two models. Neither side defines fields, attributes
/// or referential actions, which are all inferred by Prisma.
pub type ImplicitManyToManyRelationWalker<'db> = Walker<'db, ManyToManyRelationId>;

impl<'db> ImplicitManyToManyRelationWalker<'db> {
    /// Gets the relation attributes from the AST.
    fn get(&self) -> &'db Relation {
        &self.db.relations[self.id.0]
    }

    /// Is this a many-to-many self-relation?
    pub fn is_self_relation(self) -> bool {
        let rel = self.get();
        rel.model_a == rel.model_b
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
        let rel = self.get();
        match rel.attributes {
            RelationAttributes::ImplicitManyToMany { field_a, field_b: _ } => self.walk(field_a),
            _ => unreachable!(),
        }
    }

    /// The field that defines the relation in model b.
    pub fn field_b(self) -> RelationFieldWalker<'db> {
        let rel = self.get();
        match rel.attributes {
            RelationAttributes::ImplicitManyToMany { field_a: _, field_b } => self.walk(field_b),
            _ => unreachable!(),
        }
    }

    /// Traverse this relation as a generic relation.
    pub fn as_relation(self) -> RelationWalker<'db> {
        self.db.walk(self.id.0)
    }

    /// The name of the relation.
    pub fn relation_name(self) -> RelationName<'db> {
        self.field_a().relation_name()
    }

    /// The name of the column pointing to model A in the implicit join table.
    pub fn column_a_name(self) -> &'static str {
        "A"
    }

    /// The name of the column pointing to model B in the implicit join table.
    pub fn column_b_name(self) -> &'static str {
        "B"
    }

    /// A representation of the table/collection implicit in this relation.
    pub fn table_name(self) -> ImplicitManyToManyRelationTableName<'db> {
        ImplicitManyToManyRelationTableName(self.relation_name())
    }
}

/// A table name for an implicit relation's join table. Useful for its Display impl.
pub struct ImplicitManyToManyRelationTableName<'db>(RelationName<'db>);

impl Display for ImplicitManyToManyRelationTableName<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("_")?;
        Display::fmt(&self.0, f)
    }
}
