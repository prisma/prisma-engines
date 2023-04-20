mod implicit_many_to_many;
mod inline;
mod two_way_embedded_many_to_many;

pub use implicit_many_to_many::{ImplicitManyToManyRelationTableName, ImplicitManyToManyRelationWalker};
pub use inline::{CompleteInlineRelationWalker, InlineRelationWalker};
pub use two_way_embedded_many_to_many::TwoWayEmbeddedManyToManyRelationWalker;

use crate::{ast, relations::*, walkers::*};

/// A relation that has the minimal amount of information for us to create one. Useful for
/// validation purposes. Holds all possible relation types.
pub type RelationWalker<'db> = Walker<'db, RelationId>;

impl<'db> RelationWalker<'db> {
    /// The models at each end of the relation. [model A, model B]. Can be the same model twice.
    pub fn models(self) -> [ast::ModelId; 2] {
        let rel = self.get();
        [rel.model_a, rel.model_b]
    }

    /// The relation fields that define the relation. A then B.
    pub fn relation_fields(self) -> impl Iterator<Item = RelationFieldWalker<'db>> {
        let (a, b) = self.get().attributes.fields();
        [a, b].into_iter().flatten().map(move |field| self.walk(field))
    }

    /// Is any field part of the relation ignored (`@ignore`) or unsupported?
    pub fn is_ignored(self) -> bool {
        self.relation_fields().any(|f| {
            f.is_ignored()
                || f.referencing_fields()
                    .into_iter()
                    .flatten()
                    .any(|scalar_field| scalar_field.is_ignored() || scalar_field.is_unsupported())
        })
    }

    /// Is this a relation where both ends are the same model?
    pub fn is_self_relation(self) -> bool {
        let r = self.get();
        r.model_a == r.model_b
    }

    /// Converts the walker to either an implicit many to many, or a inline relation walker
    /// gathering 1:1 and 1:n relations.
    pub fn refine(self) -> RefinedRelationWalker<'db> {
        if self.get().is_implicit_many_to_many() {
            RefinedRelationWalker::ImplicitManyToMany(self.walk(ManyToManyRelationId(self.id)))
        } else if self.get().is_two_way_embedded_many_to_many() {
            RefinedRelationWalker::TwoWayEmbeddedManyToMany(TwoWayEmbeddedManyToManyRelationWalker(self))
        } else {
            RefinedRelationWalker::Inline(InlineRelationWalker(self))
        }
    }

    /// The relation name in the schema.
    ///
    /// ```ignore
    /// myField OtherModel @relation("thisModelToOtherModel", fields: [fkfield], references: [id])
    /// //                           ^^^^^^^^^^^^^^^^^^^^^^^
    /// ```
    pub fn explicit_relation_name(self) -> Option<&'db str> {
        self.get().relation_name.map(|string_id| &self.db[string_id])
    }

    /// The relation name, explicit or inferred.
    ///
    /// ```ignore
    /// posts Post[] @relation("UserPosts")
    ///                        ^^^^^^^^^^^
    /// ```
    pub fn relation_name(self) -> RelationName<'db> {
        let relation = self.get();
        relation
            .relation_name
            .map(|s| RelationName::Explicit(&self.db[s]))
            .unwrap_or_else(|| {
                RelationName::generated(self.walk(relation.model_a).name(), self.walk(relation.model_b).name())
            })
    }

    /// The relation attributes parsed from the AST.
    fn get(self) -> &'db Relation {
        &self.db.relations[self.id]
    }
}

/// Splits the relation to different types.
pub enum RefinedRelationWalker<'db> {
    /// 1:1 and 1:n relations, where one side defines the relation arguments.
    Inline(InlineRelationWalker<'db>),
    /// Implicit m:n relation. The arguments are inferred by Prisma.
    ImplicitManyToMany(ImplicitManyToManyRelationWalker<'db>),
    /// Embedded 2-way m:n relation.
    TwoWayEmbeddedManyToMany(TwoWayEmbeddedManyToManyRelationWalker<'db>),
}

impl<'db> RefinedRelationWalker<'db> {
    /// Try interpreting this relation as an inline (1:n or 1:1 — without join table) relation.
    pub fn as_inline(&self) -> Option<InlineRelationWalker<'db>> {
        match self {
            RefinedRelationWalker::Inline(inline) => Some(*inline),
            _ => None,
        }
    }

    /// Try interpreting this relation as an implicit many-to-many relation.
    pub fn as_many_to_many(&self) -> Option<ImplicitManyToManyRelationWalker<'db>> {
        match self {
            RefinedRelationWalker::ImplicitManyToMany(m2m) => Some(*m2m),
            _ => None,
        }
    }
}
