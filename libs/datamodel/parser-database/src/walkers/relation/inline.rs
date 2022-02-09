mod complete;

pub use complete::CompleteInlineRelationWalker;

use schema_ast::ast;

use crate::{
    relations::{OneToManyRelationFields, OneToOneRelationFields, Relation, RelationAttributes},
    walkers::{ModelWalker, RelationFieldWalker, RelationName, ScalarFieldWalker},
};

use super::{camel_case, pascal_case, InferredField, ReferencingFields, RelationWalker};

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
        self.0.db.walk_model(self.get().model_a)
    }

    /// The model referenced and which hold the back-relation field.
    pub fn referenced_model(self) -> ModelWalker<'db> {
        self.0.db.walk_model(self.get().model_b)
    }

    /// If the relation is defined from both sides, convert to an explicit relation
    /// walker.
    pub fn as_complete(self) -> Option<CompleteInlineRelationWalker<'db>> {
        match (self.forward_relation_field(), self.back_relation_field()) {
            (Some(field_a), Some(field_b)) => {
                let walker = CompleteInlineRelationWalker {
                    side_a: (self.referencing_model().model_id, field_a.field_id),
                    side_b: (self.referenced_model().model_id, field_b.field_id),
                    db: self.0.db,
                };

                Some(walker)
            }
            _ => None,
        }
    }

    /// Should only be used for lifting. The referencing fields (including possibly inferred ones).
    pub fn referencing_fields(self) -> ReferencingFields<'db> {
        self.forward_relation_field()
            .and_then(|rf| rf.fields())
            .map(|fields| ReferencingFields::Concrete(Box::new(fields)))
            .unwrap_or_else(|| match self.referenced_model().unique_criterias().next() {
                Some(first_unique_criteria) => {
                    let fields = first_unique_criteria
                        .fields()
                        .map(|field| {
                            let name = format!(
                                "{}{}",
                                camel_case(self.referenced_model().name()),
                                pascal_case(field.name())
                            );

                            if let Some(existing_field) =
                                self.referencing_model().scalar_fields().find(|sf| sf.name() == name)
                            {
                                InferredField {
                                    name,
                                    arity: existing_field.ast_field().arity,
                                    tpe: existing_field.scalar_field_type(),
                                    blueprint: field,
                                }
                            } else {
                                InferredField {
                                    name,
                                    arity: ast::FieldArity::Optional,
                                    tpe: field.scalar_field_type(),
                                    blueprint: field,
                                }
                            }
                        })
                        .collect();

                    ReferencingFields::Inferred(fields)
                }
                None => ReferencingFields::NA,
            })
    }

    /// Should only be used for lifting. The referenced fields. Inferred or specified.
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
                        .flat_map(|c| c.fields()),
                )
            })
    }

    /// The forward relation field (the relation field on model A, the referencing model).
    pub fn forward_relation_field(self) -> Option<RelationFieldWalker<'db>> {
        let model = self.referencing_model();
        match self.get().attributes {
            RelationAttributes::OneToOne(OneToOneRelationFields::Forward(a))
            | RelationAttributes::OneToOne(OneToOneRelationFields::Both(a, _))
            | RelationAttributes::OneToMany(OneToManyRelationFields::Both(a, _))
            | RelationAttributes::OneToMany(OneToManyRelationFields::Forward(a)) => Some(model.relation_field(a)),
            RelationAttributes::OneToMany(OneToManyRelationFields::Back(_)) => None,
            RelationAttributes::ImplicitManyToMany { field_a: _, field_b: _ } => unreachable!(),
            RelationAttributes::TwoWayEmbeddedManyToMany { field_a: _, field_b: _ } => unreachable!(),
        }
    }

    /// The arity of the forward relation field.
    pub fn forward_relation_field_arity(self) -> ast::FieldArity {
        self.forward_relation_field()
            .map(|rf| rf.ast_field().arity)
            .unwrap_or_else(|| {
                let is_required = match self.referencing_fields() {
                    ReferencingFields::Concrete(mut fields) => fields.all(|f| f.ast_field().arity.is_required()),
                    ReferencingFields::Inferred(fields) => fields.iter().all(|f| f.arity.is_required()),
                    ReferencingFields::NA => todo!(),
                };
                if is_required {
                    ast::FieldArity::Required
                } else {
                    ast::FieldArity::Optional
                }
            })
    }

    /// The contents of the `map: ...` argument of the `@relation` attribute.
    pub fn mapped_name(self) -> Option<&'db str> {
        self.forward_relation_field().and_then(|field| field.mapped_name())
    }

    /// The back relation field, or virtual relation field (on model B, the referenced model).
    pub fn back_relation_field(self) -> Option<RelationFieldWalker<'db>> {
        let model = self.referenced_model();
        match self.get().attributes {
            RelationAttributes::OneToOne(OneToOneRelationFields::Both(_, b))
            | RelationAttributes::OneToMany(OneToManyRelationFields::Both(_, b))
            | RelationAttributes::OneToMany(OneToManyRelationFields::Back(b)) => Some(model.relation_field(b)),
            RelationAttributes::OneToMany(OneToManyRelationFields::Forward(_))
            | RelationAttributes::OneToOne(OneToOneRelationFields::Forward(_)) => None,
            RelationAttributes::ImplicitManyToMany { field_a: _, field_b: _ } => unreachable!(),
            RelationAttributes::TwoWayEmbeddedManyToMany { field_a: _, field_b: _ } => unreachable!(),
        }
    }

    /// The name of the relation. Either uses the `name` (or default) argument,
    /// or generates an implicit name.
    pub fn relation_name(self) -> RelationName<'db> {
        self.get()
            .relation_name
            .map(|string_id| &self.0.db[string_id])
            .map(RelationName::Explicit)
            .unwrap_or_else(|| RelationName::generated(self.referencing_model().name(), self.referenced_model().name()))
    }
}
