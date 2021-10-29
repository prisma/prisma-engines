use super::{ModelWalker, RelationFieldWalker, RelationName, ScalarFieldWalker};
use crate::{
    ast,
    transform::ast_to_dml::db::{relations::*, ParserDatabase, ScalarFieldType},
};
use datamodel_connector::ConnectorCapability;
use dml::relation_info::ReferentialAction;

#[derive(Copy, Clone)]
pub(crate) struct RelationWalker<'ast, 'db> {
    pub(super) relation_id: usize,
    pub(super) db: &'db ParserDatabase<'ast>,
}

impl<'ast, 'db> RelationWalker<'ast, 'db> {
    pub(crate) fn refine(self) -> RefinedRelationWalker<'ast, 'db> {
        let relation = &self.db.relations.relations_storage[self.relation_id];

        if relation.is_many_to_many() {
            RefinedRelationWalker::ImplicitManyToMany(ImplicitManyToManyRelationWalker {
                db: self.db,
                relation_id: self.relation_id,
            })
        } else {
            RefinedRelationWalker::Inline(InlineRelationWalker {
                relation_id: self.relation_id,
                db: self.db,
            })
        }
    }
}

pub(crate) enum RefinedRelationWalker<'ast, 'db> {
    Inline(InlineRelationWalker<'ast, 'db>),
    ImplicitManyToMany(ImplicitManyToManyRelationWalker<'ast, 'db>),
}

/// A scalar inferred by loose/magic reformatting
pub(crate) struct InferredField<'ast, 'db> {
    pub(crate) name: String,
    pub(crate) arity: ast::FieldArity,
    pub(crate) tpe: ScalarFieldType,
    pub(crate) blueprint: ScalarFieldWalker<'ast, 'db>,
}

pub(crate) enum ReferencingFields<'ast, 'db> {
    Concrete(Box<dyn Iterator<Item = ScalarFieldWalker<'ast, 'db>> + 'db>),
    Inferred(Vec<InferredField<'ast, 'db>>),
    NA,
}

/// An explicitly defined 1:1 or 1:n relation. This walker doesn't expect the relation to be
/// defined on both sides.
#[derive(Copy, Clone)]
pub(crate) struct InlineRelationWalker<'ast, 'db> {
    relation_id: usize,
    db: &'db ParserDatabase<'ast>,
}

impl<'ast, 'db> InlineRelationWalker<'ast, 'db> {
    fn get(self) -> &'db Relation<'ast> {
        &self.db.relations.relations_storage[self.relation_id]
    }

    pub(crate) fn is_one_to_one(self) -> bool {
        matches!(self.get().attributes, RelationAttributes::OneToOne(_))
    }

    pub(crate) fn referencing_model(self) -> ModelWalker<'ast, 'db> {
        self.db.walk_model(self.get().model_a)
    }

    pub(crate) fn referenced_model(self) -> ModelWalker<'ast, 'db> {
        self.db.walk_model(self.get().model_b)
    }

    /// If the relation is defined from both sides, convert to an explicit relation
    /// walker.
    pub(crate) fn as_complete(self) -> Option<CompleteInlineRelationWalker<'ast, 'db>> {
        match (self.forward_relation_field(), self.back_relation_field()) {
            (Some(field_a), Some(field_b)) => {
                let walker = CompleteInlineRelationWalker {
                    side_a: (self.referencing_model().model_id, field_a.field_id),
                    side_b: (self.referenced_model().model_id, field_b.field_id),
                    relation: &self.db.relations.relations_storage[self.relation_id],
                    db: self.db,
                };

                Some(walker)
            }
            _ => None,
        }
    }

    // Should only be used for lifting
    pub(crate) fn referencing_fields(self) -> ReferencingFields<'ast, 'db> {
        use crate::common::NameNormalizer;

        self.forward_relation_field()
            .and_then(|rf| rf.fields())
            .map(|fields| ReferencingFields::Concrete(Box::new(fields)))
            .unwrap_or_else(|| match self.referenced_model().unique_criterias().next() {
                Some(first_unique_criteria) => ReferencingFields::Inferred(
                    first_unique_criteria
                        .fields()
                        .map(|field| {
                            let name = format!(
                                "{}{}",
                                self.referenced_model().name().camel_case(),
                                field.name().pascal_case()
                            );

                            if let Some(existing_field) =
                                self.referencing_model().scalar_fields().find(|sf| sf.name() == name)
                            {
                                InferredField {
                                    name,
                                    arity: existing_field.ast_field().arity,
                                    tpe: existing_field.attributes().r#type,
                                    blueprint: field,
                                }
                            } else {
                                InferredField {
                                    name,
                                    arity: ast::FieldArity::Optional,
                                    tpe: field.attributes().r#type,
                                    blueprint: field,
                                }
                            }
                        })
                        .collect(),
                ),
                None => ReferencingFields::NA,
            })
    }

    // Should only be used for lifting
    pub(crate) fn referenced_fields(self) -> Box<dyn Iterator<Item = ScalarFieldWalker<'ast, 'db>> + 'db> {
        self.forward_relation_field()
            .and_then(
                |field: RelationFieldWalker<'ast, 'db>| -> Option<Box<dyn Iterator<Item = ScalarFieldWalker<'ast, 'db>>>> {
                    field.referenced_fields().map(|fields| Box::new(fields) as Box<dyn Iterator<Item = ScalarFieldWalker<'ast, 'db>>>)
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

    pub(crate) fn forward_relation_field(self) -> Option<RelationFieldWalker<'ast, 'db>> {
        let model = self.referencing_model();
        match self.get().attributes {
            RelationAttributes::OneToOne(OneToOneRelationFields::Forward(a))
            | RelationAttributes::OneToOne(OneToOneRelationFields::Both(a, _))
            | RelationAttributes::OneToMany(OneToManyRelationFields::Both(a, _))
            | RelationAttributes::OneToMany(OneToManyRelationFields::Forward(a)) => Some(model.relation_field(a)),
            RelationAttributes::OneToMany(OneToManyRelationFields::Back(_)) => None,
            RelationAttributes::ImplicitManyToMany { field_a: _, field_b: _ } => unreachable!(),
        }
    }

    pub(crate) fn forward_relation_field_arity(self) -> ast::FieldArity {
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

    pub(crate) fn back_relation_field(self) -> Option<RelationFieldWalker<'ast, 'db>> {
        let model = self.referenced_model();
        match self.get().attributes {
            RelationAttributes::OneToOne(OneToOneRelationFields::Both(_, b))
            | RelationAttributes::OneToMany(OneToManyRelationFields::Both(_, b))
            | RelationAttributes::OneToMany(OneToManyRelationFields::Back(b)) => Some(model.relation_field(b)),
            RelationAttributes::OneToMany(OneToManyRelationFields::Forward(_))
            | RelationAttributes::OneToOne(OneToOneRelationFields::Forward(_)) => None,
            RelationAttributes::ImplicitManyToMany { field_a: _, field_b: _ } => unreachable!(),
        }
    }

    /// The name of the relation. Either uses the `name` (or default) argument,
    /// or generates an implicit name.
    pub(crate) fn relation_name(self) -> RelationName<'ast> {
        self.get()
            .relation_name
            .map(RelationName::Explicit)
            .unwrap_or_else(|| RelationName::generated(self.referencing_model().name(), self.referenced_model().name()))
    }
}

#[derive(Copy, Clone)]
pub(crate) struct ImplicitManyToManyRelationWalker<'ast, 'db> {
    relation_id: usize,
    db: &'db ParserDatabase<'ast>,
}

impl<'ast, 'db> ImplicitManyToManyRelationWalker<'ast, 'db> {
    fn get(&self) -> &'db Relation<'ast> {
        &self.db.relations.relations_storage[self.relation_id]
    }

    pub(crate) fn model_a(self) -> ModelWalker<'ast, 'db> {
        self.db.walk_model(self.get().model_a)
    }

    pub(crate) fn model_b(self) -> ModelWalker<'ast, 'db> {
        self.db.walk_model(self.get().model_b)
    }

    pub(crate) fn field_a(self) -> RelationFieldWalker<'ast, 'db> {
        match self.get().attributes {
            RelationAttributes::ImplicitManyToMany { field_a, field_b: _ } => self.model_a().relation_field(field_a),
            _ => unreachable!(),
        }
    }

    pub(crate) fn field_b(self) -> RelationFieldWalker<'ast, 'db> {
        match self.get().attributes {
            RelationAttributes::ImplicitManyToMany { field_a: _, field_b } => self.model_b().relation_field(field_b),
            _ => unreachable!(),
        }
    }
}

/// Represents a relation that has fields and references defined in one of the
/// relation fields. Includes 1:1 and 1:n relations that are defined from both sides.
#[derive(Copy, Clone)]
pub(crate) struct CompleteInlineRelationWalker<'ast, 'db> {
    pub(crate) side_a: (ast::ModelId, ast::FieldId),
    pub(crate) side_b: (ast::ModelId, ast::FieldId),
    pub(crate) relation: &'db Relation<'ast>,
    pub(crate) db: &'db ParserDatabase<'ast>,
}

impl<'ast, 'db> CompleteInlineRelationWalker<'ast, 'db> {
    /// The model that defines the relation fields and actions.
    pub(crate) fn referencing_model(self) -> ModelWalker<'ast, 'db> {
        ModelWalker {
            model_id: self.side_a.0,
            db: self.db,
            model_attributes: &self.db.types.model_attributes[&self.side_a.0],
        }
    }

    /// The implicit relation side.
    pub(crate) fn referenced_model(self) -> ModelWalker<'ast, 'db> {
        ModelWalker {
            model_id: self.side_b.0,
            db: self.db,
            model_attributes: &self.db.types.model_attributes[&self.side_b.0],
        }
    }

    pub(crate) fn referencing_field(self) -> RelationFieldWalker<'ast, 'db> {
        RelationFieldWalker {
            model_id: self.side_a.0,
            field_id: self.side_a.1,
            db: self.db,
            relation_field: &self.db.types.relation_fields[&(self.side_a.0, self.side_a.1)],
        }
    }

    /// The scalar fields defining the relation on the referenced model.
    pub(crate) fn referenced_fields(self) -> impl ExactSizeIterator<Item = ScalarFieldWalker<'ast, 'db>> + 'db {
        let f = move |field_id: &ast::FieldId| {
            let model_id = self.referenced_model().model_id;

            ScalarFieldWalker {
                model_id,
                field_id: *field_id,
                db: self.db,
                scalar_field: &self.db.types.scalar_fields[&(model_id, *field_id)],
            }
        };

        match self.referencing_field().relation_field.references.as_ref() {
            Some(references) => references.iter().map(f),
            None => [].iter().map(f),
        }
    }

    /// The scalar fields on the defining the relation on the referencing model.
    pub(crate) fn referencing_fields(self) -> impl ExactSizeIterator<Item = ScalarFieldWalker<'ast, 'db>> + 'db {
        let f = move |field_id: &ast::FieldId| {
            let model_id = self.referencing_model().model_id;

            ScalarFieldWalker {
                model_id,
                field_id: *field_id,
                db: self.db,
                scalar_field: &self.db.types.scalar_fields[&(model_id, *field_id)],
            }
        };

        match self.referencing_field().relation_field.fields.as_ref() {
            Some(references) => references.iter().map(f),
            None => [].iter().map(f),
        }
    }

    /// Gives the onUpdate referential action of the relation. If not defined
    /// explicitly, returns the default value.
    pub(crate) fn on_update(self) -> ReferentialAction {
        use ReferentialAction::*;

        self.referencing_field().attributes().on_update.unwrap_or_else(|| {
            let uses_foreign_keys = self
                .db
                .active_connector()
                .has_capability(ConnectorCapability::ForeignKeys);

            match self.referential_arity() {
                _ if uses_foreign_keys => Cascade,
                ast::FieldArity::Required => NoAction,
                _ => SetNull,
            }
        })
    }

    /// Gives the onDelete referential action of the relation. If not defined
    /// explicitly, returns the default value.
    pub(crate) fn on_delete(self) -> ReferentialAction {
        use ReferentialAction::*;

        self.referencing_field().attributes().on_delete.unwrap_or_else(|| {
            let supports_restrict = self.db.active_connector().supports_referential_action(Restrict);

            match self.referential_arity() {
                ast::FieldArity::Required if supports_restrict => Restrict,
                ast::FieldArity::Required => NoAction,
                _ => SetNull,
            }
        })
    }

    /// Prisma allows setting the relation field as optional, even if one of the
    /// underlying scalar fields is required. For the purpose of referential
    /// actions, we count the relation field required if any of the underlying
    /// fields is required.
    pub(crate) fn referential_arity(self) -> ast::FieldArity {
        self.referencing_field().referential_arity()
    }

    /// 1:1, 1:n or m:n
    pub(crate) fn relation_type(self) -> RelationType {
        self.relation.r#type()
    }
}
