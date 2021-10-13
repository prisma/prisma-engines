mod primary_key;
mod unique_criteria;

pub(crate) use primary_key::*;
pub(crate) use unique_criteria::*;

use super::{ExplicitRelationWalker, IndexWalker, RelationFieldWalker};
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
    pub(crate) fn name(&self) -> &'ast str {
        self.ast_model().name()
    }

    /// The ID of the model in the db
    pub(crate) fn model_id(&self) -> ast::ModelId {
        self.model_id
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
    pub(super) fn get_field_db_names<'a>(&'a self, fields: &'a [ast::FieldId]) -> impl Iterator<Item = &'ast str> + 'a {
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

    /// Iterate all the relation fields in the model in the order they were
    /// defined. Note that these are only the fields that were actually written
    /// in the schema.
    pub(crate) fn relation_fields(&self) -> impl Iterator<Item = RelationFieldWalker<'ast, 'db>> + 'db {
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
    pub(crate) fn explicit_complete_relations_fwd(
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
