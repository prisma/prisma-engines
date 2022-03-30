mod primary_key;
mod unique_criteria;

pub use primary_key::*;
pub(crate) use unique_criteria::*;

use super::{
    CompleteInlineRelationWalker, IndexWalker, InlineRelationWalker, RelationFieldWalker, RelationWalker,
    ScalarFieldWalker,
};
use crate::{
    ast::{self, WithName},
    types::ModelAttributes,
    ParserDatabase,
};
use std::hash::{Hash, Hasher};

/// A `model` declaration in the Prisma schema.
#[derive(Copy, Clone, Debug)]
pub struct ModelWalker<'db> {
    pub(super) model_id: ast::ModelId,
    pub(super) db: &'db ParserDatabase,
    pub(super) model_attributes: &'db ModelAttributes,
}

impl<'db> PartialEq for ModelWalker<'db> {
    fn eq(&self, other: &Self) -> bool {
        self.model_id == other.model_id
    }
}

impl<'db> Eq for ModelWalker<'db> {}

impl<'db> Hash for ModelWalker<'db> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.model_id.hash(state);
    }
}

impl<'db> ModelWalker<'db> {
    /// The name of the model.
    pub fn name(self) -> &'db str {
        self.ast_model().name()
    }

    /// Whether MySQL would consider the field indexed for autoincrement purposes.
    pub fn field_is_indexed_for_autoincrement(&self, field_id: ast::FieldId) -> bool {
        self.indexes()
            .any(|idx| idx.fields().next().map(|f| f.field_id()) == Some(field_id))
            || self
                .primary_key()
                .filter(|pk| pk.fields().next().map(|f| f.field_id()) == Some(field_id))
                .is_some()
    }

    /// Whether the field is the whole primary key. Will match `@id` and `@@id([fieldName])`.
    pub fn field_is_single_pk(&self, field: ast::FieldId) -> bool {
        self.primary_key()
            .filter(|pk| pk.fields().map(|f| f.field_id()).collect::<Vec<_>>() == [field])
            .is_some()
    }

    /// Is the field part of a compound primary key.
    pub fn field_is_part_of_a_compound_pk(&self, field: ast::FieldId) -> bool {
        self.primary_key()
            .filter(|pk| {
                let exists = pk.fields().map(|f| f.field_id()).any(|f| f == field);

                exists && pk.fields().len() > 1
            })
            .is_some()
    }

    /// The ID of the model in the db
    pub fn model_id(self) -> ast::ModelId {
        self.model_id
    }

    /// The AST node.
    pub fn ast_model(self) -> &'db ast::Model {
        &self.db.ast[self.model_id]
    }

    /// The parsed attributes.
    pub(crate) fn attributes(self) -> &'db ModelAttributes {
        self.model_attributes
    }

    /// Model has the @@ignore attribute.
    pub fn is_ignored(self) -> bool {
        self.attributes().is_ignored
    }

    /// The name of the database table the model points to.
    #[allow(clippy::unnecessary_lazy_evaluations)] // respectfully disagree
    pub fn database_name(self) -> &'db str {
        self.model_attributes
            .mapped_name
            .map(|id| &self.db[id])
            .unwrap_or_else(|| &self.db.ast[self.model_id].name.name)
    }

    /// Get the database name of the scalar field.
    pub fn get_field_database_name(self, field_id: ast::FieldId) -> &'db str {
        self.db.types.scalar_fields[&(self.model_id, field_id)]
            .mapped_name
            .map(|id| &self.db[id])
            .unwrap_or_else(|| &self.db.ast[self.model_id][field_id].name.name)
    }

    /// Get the database names of the constrained scalar fields.
    #[allow(clippy::unnecessary_lazy_evaluations)] // respectfully disagree
    pub fn get_field_database_names(self, fields: &'db [ast::FieldId]) -> impl Iterator<Item = &'db str> + '_ {
        fields
            .iter()
            .map(move |&field_id| self.get_field_database_name(field_id))
    }

    /// Used in validation. True only if the model has a single field id.
    pub fn has_single_id_field(self) -> bool {
        matches!(&self.attributes().primary_key, Some(pk) if pk.fields.len() == 1)
    }

    /// The name in the @@map attribute.
    pub fn mapped_name(self) -> Option<&'db str> {
        self.attributes().mapped_name.map(|id| &self.db[id])
    }

    /// The primary key of the model, if defined.
    pub fn primary_key(self) -> Option<PrimaryKeyWalker<'db>> {
        self.model_attributes.primary_key.as_ref().map(|pk| PrimaryKeyWalker {
            model_id: self.model_id,
            attribute: pk,
            db: self.db,
        })
    }

    /// Walk a scalar field by id.
    #[track_caller]
    pub(crate) fn scalar_field(&self, field_id: ast::FieldId) -> ScalarFieldWalker<'db> {
        ScalarFieldWalker {
            model_id: self.model_id,
            field_id,
            db: self.db,
            scalar_field: &self.db.types.scalar_fields[&(self.model_id, field_id)],
        }
    }

    /// Iterate all the scalar fields in a given model in the order they were defined.
    pub fn scalar_fields(self) -> impl Iterator<Item = ScalarFieldWalker<'db>> + 'db {
        let db = self.db;
        db.types
            .scalar_fields
            .range((self.model_id, ast::FieldId::MIN)..=(self.model_id, ast::FieldId::MAX))
            .map(move |((model_id, field_id), scalar_field)| ScalarFieldWalker {
                model_id: *model_id,
                field_id: *field_id,
                db,
                scalar_field,
            })
    }

    /// All unique criterias of the model; consisting of the primary key and
    /// unique indexes, if set.
    pub fn unique_criterias(self) -> impl Iterator<Item = UniqueCriteriaWalker<'db>> + 'db {
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
            .filter(|walker| walker.attribute().is_unique())
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
    pub(crate) fn explicit_indexes(self) -> impl Iterator<Item = IndexWalker<'db>> + 'db {
        let model_id = self.model_id;
        let db = self.db;

        self.model_attributes
            .ast_indexes
            .iter()
            .map(move |(index, index_attribute)| IndexWalker {
                model_id,
                index: Some(*index),
                db,
                index_attribute,
            })
    }

    /// Iterate all the indexes in the model in the order they were
    /// defined, followed by the implicit indexes.
    pub fn indexes(self) -> impl Iterator<Item = IndexWalker<'db>> + 'db {
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
    pub fn relation_fields(self) -> impl Iterator<Item = RelationFieldWalker<'db>> + 'db {
        let model_id = self.model_id;
        let db = self.db;

        self.db
            .types
            .relation_fields
            .range((model_id, ast::FieldId::MIN)..=(model_id, ast::FieldId::MAX))
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
    pub fn relation_field(self, field_id: ast::FieldId) -> RelationFieldWalker<'db> {
        RelationFieldWalker {
            model_id: self.model_id,
            field_id,
            db: self.db,
            relation_field: &self.db.types.relation_fields[&(self.model_id, field_id)],
        }
    }

    /// All relations that start from this model.
    pub fn relations_from(self) -> impl Iterator<Item = RelationWalker<'db>> + 'db {
        self.db
            .relations
            .from_model(self.model_id)
            .map(move |relation_id| RelationWalker {
                id: relation_id,
                db: self.db,
            })
    }

    /// All relations that reference this model.
    pub fn relations_to(self) -> impl Iterator<Item = RelationWalker<'db>> + 'db {
        self.db
            .relations
            .to_model(self.model_id)
            .map(move |relation_id| RelationWalker {
                id: relation_id,
                db: self.db,
            })
    }

    /// 1:n and 1:1 relations that start from this model.
    pub fn inline_relations_from(self) -> impl Iterator<Item = InlineRelationWalker<'db>> + 'db {
        self.relations_from().filter_map(|relation| match relation.refine() {
            super::RefinedRelationWalker::Inline(relation) => Some(relation),
            super::RefinedRelationWalker::ImplicitManyToMany(_) => None,
            super::RefinedRelationWalker::TwoWayEmbeddedManyToMany(_) => None,
        })
    }

    /// 1:n and 1:1 relations, starting from this model and having both sides defined.
    pub fn complete_inline_relations_from(self) -> impl Iterator<Item = CompleteInlineRelationWalker<'db>> + 'db {
        self.inline_relations_from()
            .filter_map(|relation| relation.as_complete())
    }
}
