use datamodel_connector::ConnectorCapability;
use dml::relation_info::ReferentialAction;

use super::{
    relations::Relation,
    types::{IdAttribute, ModelAttributes, RelationField},
    ParserDatabase, ScalarField,
};
use crate::{
    ast::{self, FieldArity},
    common::constraint_names::ConstraintNames,
};
use std::borrow::Cow;

impl<'ast> ParserDatabase<'ast> {
    #[track_caller]
    pub(crate) fn walk_model(&self, model_id: ast::ModelId) -> ModelWalker<'ast, '_> {
        ModelWalker {
            model_id,
            db: self,
            model_attributes: &self.types.model_attributes[&model_id],
        }
    }
}

#[derive(Copy, Clone)]
pub(crate) struct ModelWalker<'ast, 'db> {
    pub(super) model_id: ast::ModelId,
    pub(super) db: &'db ParserDatabase<'ast>,
    pub(super) model_attributes: &'db ModelAttributes<'ast>,
}

impl<'ast, 'db> ModelWalker<'ast, 'db> {
    /// The name of the model.
    pub(crate) fn name(&self) -> &'ast str {
        self.ast_model().name()
    }

    /// The AST representation.
    pub(crate) fn ast_model(&self) -> &'ast ast::Model {
        &self.db.ast[self.model_id]
    }

    /// The parsed attributes.
    pub(crate) fn attributes(&self) -> &'db ModelAttributes<'ast> {
        self.model_attributes
    }

    /// True if given fields are unique in the model.
    pub(crate) fn fields_are_unique(&self, fields: &[ast::FieldId]) -> bool {
        self.model_attributes
            .indexes
            .iter()
            .any(|(_, idx)| idx.is_unique && idx.fields == fields)
    }

    /// The name of the database table the model points to.
    #[allow(clippy::unnecessary_lazy_evaluations)] // respectfully disagree
    pub(crate) fn final_database_name(&self) -> &'ast str {
        self.model_attributes
            .mapped_name
            .unwrap_or_else(|| &self.db.ast[self.model_id].name.name)
    }

    #[allow(clippy::unnecessary_lazy_evaluations)] // respectfully disagree
    fn get_field_db_names<'a>(&'a self, fields: &'a [ast::FieldId]) -> impl Iterator<Item = &'ast str> + 'a {
        fields.iter().map(move |&field_id| {
            self.db.types.scalar_fields[&(self.model_id, field_id)]
                .mapped_name
                .unwrap_or_else(|| &self.db.ast[self.model_id][field_id].name.name)
        })
    }

    /// The primary key of the model, if defined.
    pub(crate) fn primary_key(&self) -> Option<PrimaryKeyWalker<'ast, 'db>> {
        self.model_attributes.primary_key.as_ref().map(|pk| PrimaryKeyWalker {
            model_id: self.model_id,
            attribute: pk,
            db: self.db,
        })
    }

    /// All unique criterias of the model; consisting of the primary key and
    /// unique indexes, if set.
    pub(crate) fn unique_criterias(&'db self) -> impl Iterator<Item = UniqueCriteriaWalker<'ast, 'db>> + 'db {
        let model_id = self.model_id;
        let db = self.db;

        let from_pk = self
            .model_attributes
            .primary_key
            .iter()
            .map(move |pk| UniqueCriteriaWalker {
                model_id,
                fields: &pk.fields,
                db,
            });

        let from_indices = self
            .indexes()
            .filter(|walker| walker.attribute().is_unique)
            .map(move |walker| UniqueCriteriaWalker {
                model_id,
                fields: &walker.attribute().fields,
                db,
            });

        from_pk.chain(from_indices)
    }

    /// All indexes defined in the model.
    pub(crate) fn indexes(&self) -> impl Iterator<Item = IndexWalker<'ast, 'db>> + 'db {
        let model_id = self.model_id;
        let db = self.db;

        self.model_attributes
            .indexes
            .iter()
            .map(move |(index, index_attribute)| IndexWalker {
                model_id,
                index,
                db,
                index_attribute,
            })
    }

    /// All (concrete) relation fields of the model.
    pub(crate) fn relation_fields(&self) -> impl Iterator<Item = RelationFieldWalker<'ast, 'db>> + 'db {
        let model_id = self.model_id;
        let db = self.db;

        self.db
            .iter_model_relation_fields(self.model_id)
            .map(move |(field_id, relation_field)| RelationFieldWalker {
                model_id,
                field_id,
                db,
                relation_field,
            })
    }

    /// Find a relation field with the given id.
    ///
    /// ## Panics
    ///
    /// If the field does not exist.
    pub(crate) fn relation_field(&self, field_id: ast::FieldId) -> RelationFieldWalker<'ast, 'db> {
        RelationFieldWalker {
            model_id: self.model_id,
            field_id,
            db: self.db,
            relation_field: &self.db.types.relation_fields[&(self.model_id, field_id)],
        }
    }

    /// All relations that fit in the following definition:
    ///
    /// - Is either 1:n or 1:1 relation.
    /// - Has both sides defined.
    pub(super) fn explicit_complete_relations_fwd(
        &self,
    ) -> impl Iterator<Item = ExplicitRelationWalker<'ast, 'db>> + '_ {
        self.db
            .relations
            .relations_from_model(self.model_id)
            .filter(|(_, relation)| !relation.is_many_to_many())
            .filter_map(move |(model_b, relation)| {
                relation
                    .as_complete_fields()
                    .map(|(field_a, field_b)| ExplicitRelationWalker {
                        side_a: (self.model_id, field_a),
                        side_b: (model_b, field_b),
                        db: self.db,
                        relation,
                    })
            })
    }
}

#[allow(dead_code)]
#[derive(Copy, Clone)]
pub(crate) struct IndexWalker<'ast, 'db> {
    model_id: ast::ModelId,
    index: &'ast ast::Attribute,
    db: &'db ParserDatabase<'ast>,
    index_attribute: &'db super::types::IndexAttribute<'ast>,
}

impl<'ast, 'db> IndexWalker<'ast, 'db> {
    pub(crate) fn final_database_name(&self) -> Cow<'ast, str> {
        if let Some(mapped_name) = &self.index_attribute.db_name {
            return Cow::Borrowed(mapped_name);
        }

        let model = self.db.walk_model(self.model_id);
        let model_db_name = model.final_database_name();
        let field_db_names: Vec<&str> = model.get_field_db_names(&self.index_attribute.fields).collect();

        if self.index_attribute.is_unique {
            ConstraintNames::unique_index_name(model_db_name, &field_db_names, self.db.active_connector()).into()
        } else {
            ConstraintNames::non_unique_index_name(model_db_name, &field_db_names, self.db.active_connector()).into()
        }
    }

    pub(crate) fn attribute(&self) -> &'db super::types::IndexAttribute<'ast> {
        self.index_attribute
    }
}

#[derive(Copy, Clone)]
pub(crate) struct ScalarFieldWalker<'ast, 'db> {
    model_id: ast::ModelId,
    field_id: ast::FieldId,
    db: &'db ParserDatabase<'ast>,
    scalar_field: &'db ScalarField<'ast>,
}

impl<'ast, 'db> ScalarFieldWalker<'ast, 'db> {
    #[allow(dead_code)] // we'll need this
    pub(crate) fn field_id(&self) -> ast::FieldId {
        self.field_id
    }

    pub(crate) fn ast_field(&self) -> &'ast ast::Field {
        &self.db.ast[self.model_id][self.field_id]
    }

    pub(crate) fn name(&self) -> &'ast str {
        self.ast_field().name()
    }

    pub(crate) fn is_optional(&self) -> bool {
        self.ast_field().arity.is_optional()
    }

    #[allow(dead_code)] // we'll need this
    pub(crate) fn attributes(&self) -> &'db ScalarField<'ast> {
        self.scalar_field
    }

    #[allow(dead_code)] // we'll need this
    pub(crate) fn model(&self) -> ModelWalker<'ast, 'db> {
        ModelWalker {
            model_id: self.model_id,
            db: self.db,
            model_attributes: &self.db.types.model_attributes[&self.model_id],
        }
    }
}

#[derive(Copy, Clone)]
pub(crate) struct RelationFieldWalker<'ast, 'db> {
    pub(super) model_id: ast::ModelId,
    pub(super) field_id: ast::FieldId,
    pub(super) db: &'db ParserDatabase<'ast>,
    pub(super) relation_field: &'db RelationField<'ast>,
}

impl<'ast, 'db> RelationFieldWalker<'ast, 'db> {
    pub(crate) fn field_id(&self) -> ast::FieldId {
        self.field_id
    }

    pub(crate) fn name(&self) -> &'ast str {
        self.ast_field().name()
    }

    pub(crate) fn ast_field(&self) -> &'ast ast::Field {
        &self.db.ast[self.model_id][self.field_id]
    }

    pub(crate) fn attributes(&self) -> &'db RelationField<'ast> {
        self.relation_field
    }

    pub(crate) fn model(&self) -> ModelWalker<'ast, 'db> {
        ModelWalker {
            model_id: self.model_id,
            db: self.db,
            model_attributes: &self.db.types.model_attributes[&self.model_id],
        }
    }

    pub(crate) fn related_model(&self) -> ModelWalker<'ast, 'db> {
        let model_id = self.relation_field.referenced_model;

        ModelWalker {
            model_id,
            db: self.db,
            model_attributes: &self.db.types.model_attributes[&model_id],
        }
    }

    /// This will be None for virtual relation fields (when no `fields` argument is passed).
    pub(crate) fn final_foreign_key_name(&self) -> Option<Cow<'ast, str>> {
        self.attributes().fk_name.map(Cow::Borrowed).or_else(|| {
            let fields = self.relation_field.fields.as_ref()?;
            let model = self.db.walk_model(self.model_id);
            let table_name = model.final_database_name();
            let column_names: Vec<&str> = model.get_field_db_names(fields).collect();

            Some(
                ConstraintNames::foreign_key_constraint_name(table_name, &column_names, self.db.active_connector())
                    .into(),
            )
        })
    }
}

#[derive(Copy, Clone)]
pub(crate) struct UniqueCriteriaWalker<'ast, 'db> {
    model_id: ast::ModelId,
    fields: &'db [ast::FieldId],
    db: &'db ParserDatabase<'ast>,
}

impl<'ast, 'db> UniqueCriteriaWalker<'ast, 'db> {
    pub(crate) fn fields(&'db self) -> impl ExactSizeIterator<Item = ScalarFieldWalker<'ast, 'db>> + 'db {
        self.fields.iter().map(move |field_id| ScalarFieldWalker {
            model_id: self.model_id,
            field_id: *field_id,
            db: self.db,
            scalar_field: &self.db.types.scalar_fields[&(self.model_id, *field_id)],
        })
    }
}

#[derive(Copy, Clone)]
pub(crate) struct PrimaryKeyWalker<'ast, 'db> {
    model_id: ast::ModelId,
    attribute: &'db IdAttribute<'ast>,
    db: &'db ParserDatabase<'ast>,
}

impl<'ast, 'db> PrimaryKeyWalker<'ast, 'db> {
    pub(crate) fn final_database_name(&self) -> Option<Cow<'ast, str>> {
        if !self.db.active_connector().supports_named_primary_keys() {
            return None;
        }

        Some(self.attribute.db_name.map(Cow::Borrowed).unwrap_or_else(|| {
            ConstraintNames::primary_key_name(
                self.db.walk_model(self.model_id).final_database_name(),
                self.db.active_connector(),
            )
            .into()
        }))
    }

    pub(crate) fn is_defined_on_field(&self) -> bool {
        self.attribute.source_field.is_some()
    }

    pub(crate) fn iter_ast_fields(&self) -> impl Iterator<Item = &'ast ast::Field> + '_ {
        self.attribute
            .fields
            .iter()
            .map(move |id| &self.db.ast[self.model_id][*id])
    }

    pub(crate) fn name(&self) -> Option<&'ast str> {
        self.attribute.name
    }
}

#[derive(Copy, Clone)]
pub(crate) struct ExplicitRelationWalker<'ast, 'db> {
    pub(crate) side_a: (ast::ModelId, ast::FieldId),
    pub(crate) side_b: (ast::ModelId, ast::FieldId),
    #[allow(dead_code)]
    pub(super) relation: &'db Relation<'ast>,
    pub(crate) db: &'db ParserDatabase<'ast>,
}

impl<'ast, 'db> ExplicitRelationWalker<'ast, 'db> {
    pub(super) fn referencing_model(&self) -> ModelWalker<'ast, 'db> {
        ModelWalker {
            model_id: self.side_a.0,
            db: self.db,
            model_attributes: &self.db.types.model_attributes[&self.side_a.0],
        }
    }

    pub(super) fn referenced_model(&self) -> ModelWalker<'ast, 'db> {
        ModelWalker {
            model_id: self.side_b.0,
            db: self.db,
            model_attributes: &self.db.types.model_attributes[&self.side_b.0],
        }
    }

    pub(super) fn referencing_field(&self) -> RelationFieldWalker<'ast, 'db> {
        RelationFieldWalker {
            model_id: self.side_a.0,
            field_id: self.side_a.1,
            db: self.db,
            relation_field: &self.db.types.relation_fields[&(self.side_a.0, self.side_a.1)],
        }
    }

    /// The scalar fields defining the relation on the referenced model.
    pub(super) fn referenced_fields(&'db self) -> impl ExactSizeIterator<Item = ScalarFieldWalker<'ast, 'db>> + 'db {
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
    pub(super) fn referencing_fields(&'db self) -> impl ExactSizeIterator<Item = ScalarFieldWalker<'ast, 'db>> + 'db {
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

    /// True if the relation uses more than one scalar field as the key.
    pub(super) fn is_compound(&self) -> bool {
        self.referencing_fields().len() > 1
    }

    /// Gives the onUpdate referential action of the relation. If not defined
    /// explicitly, returns the default value.
    pub(super) fn on_update(&self) -> ReferentialAction {
        use ReferentialAction::*;

        self.referencing_field().attributes().on_update.unwrap_or_else(|| {
            let uses_foreign_keys = self
                .db
                .active_connector()
                .has_capability(ConnectorCapability::ForeignKeys);

            match self.referential_arity() {
                _ if uses_foreign_keys => Cascade,
                FieldArity::Required => NoAction,
                _ => SetNull,
            }
        })
    }

    /// Gives the onDelete referential action of the relation. If not defined
    /// explicitly, returns the default value.
    pub(super) fn on_delete(&self) -> ReferentialAction {
        use ReferentialAction::*;

        self.referencing_field().attributes().on_delete.unwrap_or_else(|| {
            let supports_restrict = self.db.active_connector().supports_referential_action(Restrict);

            match self.referential_arity() {
                FieldArity::Required if supports_restrict => Restrict,
                FieldArity::Required => NoAction,
                _ => SetNull,
            }
        })
    }

    /// Prisma allows setting the relation field as optional, even if one of the
    /// underlying scalar fields is required. For the purpose of referential
    /// actions, we count the relation field required if any of the underlying
    /// fields is required.
    pub(crate) fn referential_arity(&self) -> FieldArity {
        let some_required = self.referencing_fields().any(|f| f.ast_field().arity.is_required());

        if some_required {
            FieldArity::Required
        } else {
            self.referencing_field().ast_field().arity
        }
    }
}
