use super::{
    types::{IdAttribute, ModelAttributes, RelationField},
    ParserDatabase, ScalarField,
};
use crate::{
    ast::{self, FieldId, Model},
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

    pub(crate) fn walk_models(&self) -> impl Iterator<Item = ModelWalker<'ast, '_>> + '_ {
        self.ast.iter_models().map(move |(model_id, _)| ModelWalker {
            model_id,
            db: self,
            model_attributes: &self.types.model_attributes[&model_id],
        })
    }
}

#[derive(Copy, Clone)]
pub(crate) struct ModelWalker<'ast, 'db> {
    pub(super) model_id: ast::ModelId,
    pub(super) db: &'db ParserDatabase<'ast>,
    pub(super) model_attributes: &'db ModelAttributes<'ast>,
}

impl<'ast, 'db> ModelWalker<'ast, 'db> {
    pub(crate) fn ast_model(&self) -> &'db Model {
        &self.db.ast[self.model_id]
    }

    pub(crate) fn attributes(&self) -> &'db ModelAttributes<'ast> {
        self.model_attributes
    }

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

    pub(crate) fn primary_key(&self) -> Option<PrimaryKeyWalker<'ast, 'db>> {
        self.model_attributes.primary_key.as_ref().map(|pk| PrimaryKeyWalker {
            model_id: self.model_id,
            attribute: pk,
            db: self.db,
        })
    }

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
            .walk_indexes()
            .filter(|walker| walker.attribute().is_unique)
            .map(move |walker| UniqueCriteriaWalker {
                model_id,
                fields: &walker.attribute().fields,
                db,
            });

        from_pk.chain(from_indices)
    }

    pub(crate) fn walk_indexes(&self) -> impl Iterator<Item = IndexWalker<'ast, 'db>> + 'db {
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

    pub(crate) fn walk_relation_fields(&self) -> impl Iterator<Item = RelationFieldWalker<'ast, 'db>> + 'db {
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

    pub(crate) fn walk_relation_field(&self, field_id: FieldId) -> RelationFieldWalker<'ast, 'db> {
        RelationFieldWalker {
            model_id: self.model_id,
            field_id,
            db: self.db,
            relation_field: &self.db.types.relation_fields[&(self.model_id, field_id)],
        }
    }
}

#[derive(Copy, Clone)]
pub(crate) struct IndexWalker<'ast, 'db> {
    model_id: ast::ModelId,
    index: &'ast ast::Attribute,
    db: &'db ParserDatabase<'ast>,
    index_attribute: &'db super::types::IndexAttribute<'ast>,
}

impl<'ast, 'db> IndexWalker<'ast, 'db> {
    pub(crate) fn ast_attribute(&self) -> &'ast ast::Attribute {
        self.index
    }

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
    model_id: ast::ModelId,
    field_id: ast::FieldId,
    db: &'db ParserDatabase<'ast>,
    relation_field: &'db RelationField<'ast>,
}

impl<'ast, 'db> RelationFieldWalker<'ast, 'db> {
    pub(crate) fn field_id(&self) -> ast::FieldId {
        self.field_id
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

    pub(crate) fn referencing_fields(&'db self) -> impl ExactSizeIterator<Item = ScalarFieldWalker<'ast, 'db>> + 'db {
        let f = move |field_id: &FieldId| ScalarFieldWalker {
            model_id: self.model_id,
            field_id: *field_id,
            db: self.db,
            scalar_field: &self.db.types.scalar_fields[&(self.model_id, *field_id)],
        };

        match self.relation_field.fields.as_ref() {
            Some(references) => references.iter().map(f),
            None => [].iter().map(f),
        }
    }

    #[allow(dead_code)]
    pub(crate) fn referenced_fields(&'db self) -> impl ExactSizeIterator<Item = ScalarFieldWalker<'ast, 'db>> + 'db {
        let f = move |field_id: &FieldId| {
            let model_id = self.attributes().referenced_model;

            ScalarFieldWalker {
                model_id,
                field_id: *field_id,
                db: self.db,
                scalar_field: &self.db.types.scalar_fields[&(model_id, *field_id)],
            }
        };

        match self.relation_field.references.as_ref() {
            Some(references) => references.iter().map(f),
            None => [].iter().map(f),
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
    fields: &'db [FieldId],
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
