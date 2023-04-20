mod complete;

pub use complete::CompleteInlineRelationWalker;

use super::RelationWalker;
use crate::{
    relations::{OneToManyRelationFields, OneToOneRelationFields, Relation, RelationAttributes},
    walkers::*,
};

/// An explicitly defined 1:1 or 1:n relation. The walker has the referencing side defined, but
/// might miss the back relation in the AST.
#[derive(Copy, Clone)]
pub struct InlineRelationWalker<'db>(pub(super) RelationWalker<'db>);

impl<'db> InlineRelationWalker<'db> {
    /// Get the relation attributes defined in the AST.
    fn get(self) -> &'db Relation {
        &self.0.db.relations[self.0.id]
    }

    /// The relation is 1:1, having at most one record on both sides of the relation.
    pub fn is_one_to_one(self) -> bool {
        matches!(self.get().attributes, RelationAttributes::OneToOne(_))
    }

    /// The model which holds the relation arguments.
    pub fn referencing_model(self) -> ModelWalker<'db> {
        self.0.db.walk(self.get().model_a)
    }

    /// The model referenced and which hold the back-relation field.
    pub fn referenced_model(self) -> ModelWalker<'db> {
        self.0.db.walk(self.get().model_b)
    }

    /// If the relation is defined from both sides, convert to an explicit relation
    /// walker.
    pub fn as_complete(self) -> Option<CompleteInlineRelationWalker<'db>> {
        match (self.forward_relation_field(), self.back_relation_field()) {
            (Some(field_a), Some(field_b)) => {
                let walker = CompleteInlineRelationWalker {
                    side_a: field_a.id,
                    side_b: field_b.id,
                    db: self.0.db,
                };

                Some(walker)
            }
            _ => None,
        }
    }

    /// The referencing fields, from the forward relation field.
    pub fn referencing_fields(self) -> Option<impl ExactSizeIterator<Item = ScalarFieldWalker<'db>>> {
        self.forward_relation_field().and_then(|rf| rf.fields())
    }

    /// The referenced fields. Inferred or specified.
    pub fn referenced_fields(self) -> Box<dyn Iterator<Item = ScalarFieldWalker<'db>> + 'db> {
        self.forward_relation_field()
            .and_then(
                |field: RelationFieldWalker<'db>| -> Option<Box<dyn Iterator<Item = ScalarFieldWalker<'db>>>> {
                    field
                        .referenced_fields()
                        .map(|fields| Box::new(fields) as Box<dyn Iterator<Item = ScalarFieldWalker<'db>>>)
                },
            )
            .unwrap_or_else(move || {
                Box::new(
                    self.referenced_model()
                        .unique_criterias()
                        .find(|c| c.is_strict_criteria())
                        .into_iter()
                        .flat_map(|c| c.fields())
                        .filter_map(|f| f.as_scalar_field()),
                )
            })
    }

    /// The forward relation field (the relation field on model A, the referencing model).
    pub fn forward_relation_field(self) -> Option<RelationFieldWalker<'db>> {
        let rel = self.get();
        match rel.attributes {
            RelationAttributes::OneToOne(OneToOneRelationFields::Forward(a))
            | RelationAttributes::OneToOne(OneToOneRelationFields::Both(a, _))
            | RelationAttributes::OneToMany(OneToManyRelationFields::Both(a, _))
            | RelationAttributes::OneToMany(OneToManyRelationFields::Forward(a)) => Some(self.0.walk(a)),
            RelationAttributes::OneToMany(OneToManyRelationFields::Back(_)) => None,
            RelationAttributes::ImplicitManyToMany { field_a: _, field_b: _ } => unreachable!(),
            RelationAttributes::TwoWayEmbeddedManyToMany { field_a: _, field_b: _ } => unreachable!(),
        }
    }

    /// The contents of the `map: ...` argument of the `@relation` attribute.
    pub fn mapped_name(self) -> Option<&'db str> {
        self.forward_relation_field().and_then(|field| field.mapped_name())
    }

    /// The back relation field, or virtual relation field (on model B, the referenced model).
    pub fn back_relation_field(self) -> Option<RelationFieldWalker<'db>> {
        let rel = self.get();
        match rel.attributes {
            RelationAttributes::OneToOne(OneToOneRelationFields::Both(_, b))
            | RelationAttributes::OneToMany(OneToManyRelationFields::Both(_, b))
            | RelationAttributes::OneToMany(OneToManyRelationFields::Back(b)) => Some(self.0.walk(b)),
            RelationAttributes::OneToMany(OneToManyRelationFields::Forward(_))
            | RelationAttributes::OneToOne(OneToOneRelationFields::Forward(_)) => None,
            RelationAttributes::ImplicitManyToMany { field_a: _, field_b: _ } => unreachable!(),
            RelationAttributes::TwoWayEmbeddedManyToMany { field_a: _, field_b: _ } => unreachable!(),
        }
    }

    /// The unique identifier of the relation.
    pub fn relation_id(self) -> crate::RelationId {
        self.0.id
    }

    /// The relation name in the schema.
    ///
    /// ```ignore
    /// myField OtherModel @relation("thisModelToOtherModel", fields: [fkfield], references: [id])
    /// //                           ^^^^^^^^^^^^^^^^^^^^^^^
    /// ```
    pub fn explicit_relation_name(self) -> Option<&'db str> {
        self.0.explicit_relation_name()
    }

    /// The name of the relation. Either uses the `name` (or default) argument,
    /// or generates an implicit name.
    pub fn relation_name(self) -> RelationName<'db> {
        self.explicit_relation_name()
            .map(RelationName::Explicit)
            .unwrap_or_else(|| RelationName::generated(self.referencing_model().name(), self.referenced_model().name()))
    }
}
