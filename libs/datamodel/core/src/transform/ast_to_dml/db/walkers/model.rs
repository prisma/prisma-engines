mod primary_key;
mod unique_criteria;

pub(crate) use primary_key::*;
pub(crate) use unique_criteria::*;

use super::{
    CompleteInlineRelationWalker, IndexWalker, InlineRelationWalker, RelationFieldWalker, RelationWalker,
    ScalarFieldWalker,
};
use crate::{
    ast,
    transform::ast_to_dml::db::{types::ModelAttributes, ParserDatabase},
};
use std::hash::{Hash, Hasher};

#[derive(Copy, Clone)]
pub(crate) struct ModelWalker<'ast, 'db> {
    pub(super) model_id: ast::ModelId,
    pub(super) db: &'db ParserDatabase<'ast>,
    pub(super) model_attributes: &'db ModelAttributes<'ast>,
}

impl<'ast, 'db> PartialEq for ModelWalker<'ast, 'db> {
    fn eq(&self, other: &Self) -> bool {
        self.model_id == other.model_id
    }
}

impl<'ast, 'db> Eq for ModelWalker<'ast, 'db> {}

impl<'ast, 'db> Hash for ModelWalker<'ast, 'db> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.model_id.hash(state);
    }
}

impl<'ast, 'db> ModelWalker<'ast, 'db> {
    /// The name of the model.
    pub(crate) fn name(self) -> &'ast str {
        self.ast_model().name()
    }

    /// The ID of the model in the db
    pub(crate) fn model_id(self) -> ast::ModelId {
        self.model_id
    }

    /// The AST representation.
    pub(crate) fn ast_model(self) -> &'ast ast::Model {
        &self.db.ast[self.model_id]
    }

    /// The parsed attributes.
    pub(crate) fn attributes(self) -> &'db ModelAttributes<'ast> {
        self.model_attributes
    }

    /// Model has the @@ignore attribute.
    pub(crate) fn is_ignored(self) -> bool {
        self.attributes().is_ignored
    }

    /// True if given fields are unique in the model.
    pub(crate) fn fields_are_unique(self, fields: &[ast::FieldId]) -> bool {
        self.model_attributes
            .ast_indexes
            .iter()
            .any(|(_, idx)| idx.is_unique && idx.fields == fields)
    }

    /// The name of the database table the model points to.
    #[allow(clippy::unnecessary_lazy_evaluations)] // respectfully disagree
    pub(crate) fn final_database_name(self) -> &'ast str {
        self.model_attributes
            .mapped_name
            .unwrap_or_else(|| &self.db.ast[self.model_id].name.name)
    }

    #[allow(clippy::unnecessary_lazy_evaluations)] // respectfully disagree
    pub(super) fn get_field_db_names<'a>(&'a self, fields: &'a [ast::FieldId]) -> impl Iterator<Item = &'ast str> + 'a {
        fields.iter().map(move |&field_id| {
            self.db.types.scalar_fields[&(self.model_id, field_id)]
                .mapped_name
                .unwrap_or_else(|| &self.db.ast[self.model_id][field_id].name.name)
        })
    }

    /// Used in validation. True only if the model has a single field id.
    pub(crate) fn has_single_id_field(self) -> bool {
        matches!(&self.attributes().primary_key, Some(pk) if pk.fields.len() == 1)
    }

    /// The primary key of the model, if defined.
    pub(crate) fn primary_key(self) -> Option<PrimaryKeyWalker<'ast, 'db>> {
        self.model_attributes.primary_key.as_ref().map(|pk| PrimaryKeyWalker {
            model_id: self.model_id,
            attribute: pk,
            db: self.db,
        })
    }

    /// Walk a scalar field by id.
    #[track_caller]
    pub(crate) fn scalar_field(&self, field_id: ast::FieldId) -> ScalarFieldWalker<'ast, 'db> {
        ScalarFieldWalker {
            model_id: self.model_id,
            field_id,
            db: self.db,
            scalar_field: &self.db.types.scalar_fields[&(self.model_id, field_id)],
        }
    }

    /// Iterate all the scalar fields in a given model in the order they were defined.
    pub(crate) fn scalar_fields(self) -> impl Iterator<Item = ScalarFieldWalker<'ast, 'db>> + 'db {
        let db = self.db;
        db.types
            .scalar_fields
            .range((self.model_id, ast::FieldId::ZERO)..=(self.model_id, ast::FieldId::MAX))
            .map(move |((model_id, field_id), scalar_field)| ScalarFieldWalker {
                model_id: *model_id,
                field_id: *field_id,
                db,
                scalar_field,
            })
    }

    /// All unique criterias of the model; consisting of the primary key and
    /// unique indexes, if set.
    pub(crate) fn unique_criterias(self) -> impl Iterator<Item = UniqueCriteriaWalker<'ast, 'db>> + 'db {
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

    /// Iterate all the relation fields in the model in the order they were
    /// defined. Note that these are only the fields that were actually written
    /// in the schema.
    pub(crate) fn explicit_indexes(self) -> impl Iterator<Item = IndexWalker<'ast, 'db>> + 'db {
        let model_id = self.model_id;
        let db = self.db;

        self.model_attributes
            .ast_indexes
            .iter()
            .map(move |(index, index_attribute)| IndexWalker {
                model_id,
                index: Some(index),
                db,
                index_attribute,
            })
    }

    /// Iterate all the indexes in the model in the order they were
    /// defined, followed by the implicit indexes.
    pub(crate) fn indexes(self) -> impl Iterator<Item = IndexWalker<'ast, 'db>> + 'db {
        let implicit_indexes = self
            .model_attributes
            .implicit_indexes
            .iter()
            .map(move |index_attribute| IndexWalker {
                model_id: self.model_id(),
                index: None,
                db: self.db,
                index_attribute,
            });

        self.explicit_indexes().chain(implicit_indexes)
    }

    /// All (concrete) relation fields of the model.
    pub(crate) fn relation_fields(self) -> impl Iterator<Item = RelationFieldWalker<'ast, 'db>> + 'db {
        let model_id = self.model_id;
        let db = self.db;

        self.db
            .types
            .relation_fields
            .range((model_id, ast::FieldId::ZERO)..=(model_id, ast::FieldId::MAX))
            .map(move |((_, field_id), relation_field)| RelationFieldWalker {
                model_id,
                field_id: *field_id,
                db,
                relation_field,
            })
    }

    /// Find a relation field with the given id.
    ///
    /// ## Panics
    ///
    /// If the field does not exist.
    pub(crate) fn relation_field(self, field_id: ast::FieldId) -> RelationFieldWalker<'ast, 'db> {
        RelationFieldWalker {
            model_id: self.model_id,
            field_id,
            db: self.db,
            relation_field: &self.db.types.relation_fields[&(self.model_id, field_id)],
        }
    }

    /// All relations that start from this model.
    pub(crate) fn relations_from(self) -> impl Iterator<Item = RelationWalker<'ast, 'db>> + 'db {
        self.db
            .relations
            .from_model(self.model_id)
            .map(move |relation_id| RelationWalker {
                relation_id,
                db: self.db,
            })
    }

    /// 1:n and 1:1 relations that start from this model.
    pub(crate) fn inline_relations_from(self) -> impl Iterator<Item = InlineRelationWalker<'ast, 'db>> + 'db {
        self.relations_from().filter_map(|relation| match relation.refine() {
            super::RefinedRelationWalker::Inline(relation) => Some(relation),
            super::RefinedRelationWalker::ImplicitManyToMany(_) => None,
        })
    }

    /// 1:n and 1:1 relations, starting from this model and having both sides defined.
    pub(crate) fn complete_inline_relations_from(
        self,
    ) -> impl Iterator<Item = CompleteInlineRelationWalker<'ast, 'db>> + 'db {
        self.inline_relations_from()
            .filter_map(|relation| relation.as_complete())
    }
}
