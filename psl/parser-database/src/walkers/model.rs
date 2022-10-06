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
};
use schema_ast::ast::{IndentationType, NewlineType, WithSpan};

/// A `model` declaration in the Prisma schema.
pub type ModelWalker<'db> = super::Walker<'db, ast::ModelId>;

impl<'db> ModelWalker<'db> {
    /// The name of the model.
    pub fn name(self) -> &'db str {
        self.ast_model().name()
    }

    /// Whether MySQL would consider the field indexed for autoincrement purposes.
    pub fn field_is_indexed_for_autoincrement(self, field_id: ast::FieldId) -> bool {
        self.indexes()
            .any(|idx| idx.fields().next().map(|f| f.field_id()) == Some(field_id))
            || self
                .primary_key()
                .filter(|pk| pk.fields().next().map(|f| f.field_id()) == Some(field_id))
                .is_some()
    }

    /// Whether the field is the whole primary key. Will match `@id` and `@@id([fieldName])`.
    pub fn field_is_single_pk(self, field: ast::FieldId) -> bool {
        self.primary_key()
            .filter(|pk| pk.fields().map(|f| f.field_id()).collect::<Vec<_>>() == [field])
            .is_some()
    }

    /// Is the field part of a compound primary key.
    pub fn field_is_part_of_a_compound_pk(self, field: ast::FieldId) -> bool {
        self.primary_key()
            .filter(|pk| {
                let exists = pk.fields().map(|f| f.field_id()).any(|f| f == field);

                exists && pk.fields().len() > 1
            })
            .is_some()
    }

    /// The ID of the model in the db
    pub fn model_id(self) -> ast::ModelId {
        self.id
    }

    /// The AST node.
    pub fn ast_model(self) -> &'db ast::Model {
        &self.db.ast[self.id]
    }

    /// The parsed attributes.
    pub(crate) fn attributes(self) -> &'db ModelAttributes {
        &self.db.types.model_attributes[&self.id]
    }

    /// Model has the @@ignore attribute.
    pub fn is_ignored(self) -> bool {
        self.attributes().is_ignored
    }

    /// The name of the database table the model points to.
    #[allow(clippy::unnecessary_lazy_evaluations)] // respectfully disagree
    pub fn database_name(self) -> &'db str {
        self.attributes()
            .mapped_name
            .map(|id| &self.db[id])
            .unwrap_or_else(|| self.db.ast[self.id].name())
    }

    /// Get the database name of the scalar field.
    pub fn get_field_database_name(self, field_id: ast::FieldId) -> &'db str {
        self.db.types.scalar_fields[&(self.id, field_id)]
            .mapped_name
            .map(|id| &self.db[id])
            .unwrap_or_else(|| self.db.ast[self.id][field_id].name())
    }

    /// Get the database names of the constrained scalar fields.
    #[allow(clippy::unnecessary_lazy_evaluations)] // respectfully disagree
    pub fn get_field_database_names(self, fields: &'db [ast::FieldId]) -> impl Iterator<Item = &'db str> {
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
        self.attributes().primary_key.as_ref().map(|pk| PrimaryKeyWalker {
            model_id: self.id,
            attribute: pk,
            db: self.db,
        })
    }

    /// Walk a scalar field by id.
    #[track_caller]
    pub(crate) fn scalar_field(self, field_id: ast::FieldId) -> ScalarFieldWalker<'db> {
        ScalarFieldWalker {
            model_id: self.id,
            field_id,
            db: self.db,
            scalar_field: &self.db.types.scalar_fields[&(self.id, field_id)],
        }
    }

    /// Iterate all the scalar fields in a given model in the order they were defined.
    pub fn scalar_fields(self) -> impl Iterator<Item = ScalarFieldWalker<'db>> {
        let db = self.db;
        db.types
            .scalar_fields
            .range((self.id, ast::FieldId::MIN)..=(self.id, ast::FieldId::MAX))
            .map(move |((model_id, field_id), scalar_field)| ScalarFieldWalker {
                model_id: *model_id,
                field_id: *field_id,
                db,
                scalar_field,
            })
    }

    /// All unique criterias of the model; consisting of the primary key and
    /// unique indexes, if set.
    pub fn unique_criterias(self) -> impl Iterator<Item = UniqueCriteriaWalker<'db>> {
        let model_id = self.id;
        let db = self.db;

        let from_pk = self
            .attributes()
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

    /// Iterate all the indexes in the model in the order they were
    /// defined.
    pub fn indexes(self) -> impl Iterator<Item = IndexWalker<'db>> {
        let model_id = self.id;
        let db = self.db;

        self.attributes()
            .ast_indexes
            .iter()
            .map(move |(index, index_attribute)| IndexWalker {
                model_id,
                index: *index,
                db,
                index_attribute,
            })
    }

    /// All (concrete) relation fields of the model.
    pub fn relation_fields(self) -> impl Iterator<Item = RelationFieldWalker<'db>> {
        let model_id = self.id;
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
            model_id: self.id,
            field_id,
            db: self.db,
            relation_field: &self.db.types.relation_fields[&(self.id, field_id)],
        }
    }

    /// All relations that start from this model.
    pub fn relations_from(self) -> impl Iterator<Item = RelationWalker<'db>> {
        self.db
            .relations
            .from_model(self.id)
            .map(move |relation_id| RelationWalker {
                id: relation_id,
                db: self.db,
            })
    }

    /// All relations that reference this model.
    pub fn relations_to(self) -> impl Iterator<Item = RelationWalker<'db>> {
        self.db
            .relations
            .to_model(self.id)
            .map(move |relation_id| RelationWalker {
                id: relation_id,
                db: self.db,
            })
    }

    /// 1:n and 1:1 relations that start from this model.
    pub fn inline_relations_from(self) -> impl Iterator<Item = InlineRelationWalker<'db>> {
        self.relations_from().filter_map(|relation| match relation.refine() {
            super::RefinedRelationWalker::Inline(relation) => Some(relation),
            super::RefinedRelationWalker::ImplicitManyToMany(_) => None,
            super::RefinedRelationWalker::TwoWayEmbeddedManyToMany(_) => None,
        })
    }

    /// 1:n and 1:1 relations, starting from this model and having both sides defined.
    pub fn complete_inline_relations_from(self) -> impl Iterator<Item = CompleteInlineRelationWalker<'db>> {
        self.inline_relations_from()
            .filter_map(|relation| relation.as_complete())
    }

    /// How fields and arguments are indented in the model.
    pub fn indentation(self) -> IndentationType {
        let field = match self.scalar_fields().last() {
            Some(field) => field,
            None => return IndentationType::default(),
        };

        let src = self.db.source();
        let start = field.ast_field().span().start;

        let mut spaces = 0;

        for i in (0..start).rev() {
            if src.is_char_boundary(i) {
                match src[i..].chars().next() {
                    Some('\t') => return IndentationType::Tabs,
                    Some(' ') => spaces += 1,
                    _ => return IndentationType::Spaces(spaces),
                }
            }
        }

        IndentationType::default()
    }

    /// What kind of newlines the model uses.
    pub fn newline(self) -> NewlineType {
        let field = match self.scalar_fields().last() {
            Some(field) => field,
            None => return NewlineType::default(),
        };

        let src = self.db.source();
        let start = field.ast_field().span().end - 2;

        match src.chars().nth(start) {
            Some('\r') => NewlineType::Windows,
            _ => NewlineType::Unix,
        }
    }

    /// The name of the schema the model belongs to.
    ///
    /// ```ignore
    /// @@schema("public")
    ///          ^^^^^^^^
    /// ```
    pub fn schema(self) -> Option<(&'db str, ast::Span)> {
        self.attributes().schema.map(|(id, span)| (&self.db[id], span))
    }

    /// The name of the schema the model belongs to.
    ///
    /// ```ignore
    /// @@schema("public")
    ///          ^^^^^^^^
    /// ```
    pub fn schema_name(self) -> Option<&'db str> {
        self.schema().map(|(name, _)| name)
    }
}
