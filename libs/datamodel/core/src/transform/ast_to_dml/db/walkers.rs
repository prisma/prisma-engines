mod composite_type;
mod field;
mod index;
mod model;
mod relation;
mod relation_field;
mod scalar_field;

pub(crate) use composite_type::*;
pub(crate) use field::*;
pub(crate) use index::*;
pub(crate) use model::*;
pub(crate) use relation::*;
pub(crate) use relation_field::*;
pub(crate) use scalar_field::*;

use super::ParserDatabase;
use crate::ast;

impl<'ast> ParserDatabase<'ast> {
    pub(crate) fn walk_model(&self, model_id: ast::ModelId) -> ModelWalker<'ast, '_> {
        ModelWalker {
            model_id,
            db: self,
            model_attributes: &self.types.model_attributes[&model_id],
        }
    }

    pub(crate) fn walk_models(&self) -> impl Iterator<Item = ModelWalker<'ast, '_>> + '_ {
        self.ast()
            .iter_tops()
            .filter_map(|(top_id, _)| top_id.as_model_id())
            .map(move |model_id| self.walk_model(model_id))
    }

    pub(crate) fn walk_composite_type(&self, ctid: ast::CompositeTypeId) -> CompositeTypeWalker<'ast, '_> {
        CompositeTypeWalker { ctid, db: self }
    }

    pub(crate) fn walk_composite_types(&self) -> impl Iterator<Item = CompositeTypeWalker<'ast, '_>> + '_ {
        self.ast()
            .iter_tops()
            .filter_map(|(top_id, _)| top_id.as_composite_type_id())
            .map(move |ctid| CompositeTypeWalker { ctid, db: self })
    }

    pub(crate) fn walk_relations(&self) -> impl Iterator<Item = RelationWalker<'ast, '_>> + '_ {
        (0..self.relations.relations_storage.len()).map(move |relation_id| RelationWalker { db: self, relation_id })
    }

    /// Iterate all complete relations that are not many to many and are
    /// correctly defined from both sides.
    #[track_caller]
    pub(crate) fn walk_explicit_relations(&self) -> impl Iterator<Item = CompleteInlineRelationWalker<'ast, '_>> + '_ {
        self.relations
            .iter_relations()
            .filter(|(_, _, relation)| !relation.is_many_to_many())
            .filter_map(move |(model_a, model_b, relation)| {
                relation
                    .as_complete_fields()
                    .map(|(field_a, field_b)| CompleteInlineRelationWalker {
                        side_a: (model_a, field_a),
                        side_b: (model_b, field_b),
                        db: self,
                        relation,
                    })
            })
    }
}
