//! Convenient access to a datamodel as understood by ParserDatabase.
//!
//! The walkers:
//! - Know about specific types and what kind they are (models, enums, etc.)
//! - Know about attributes and which ones are defined and allowed in a Prisma schema.
//! - Know about relations.
//! - Do not know anything about connectors, they are generic.

mod composite_type;
mod r#enum;
mod field;
mod index;
mod model;
mod position;
mod relation;
mod relation_field;
mod scalar_field;

pub use composite_type::*;
pub use field::*;
pub use index::*;
pub use model::*;
pub use position::*;
pub use r#enum::*;
pub use relation::*;
pub use relation_field::*;
pub use scalar_field::*;

use crate::{ast, ParserDatabase};

/// A generic walker. Only walkers intantiated with a concrete ID type (`I`) are useful.
#[derive(Clone, Copy)]
pub struct Walker<'ast, 'db, I> {
    db: &'db crate::ParserDatabase<'ast>,
    id: I,
}

impl<'ast> ParserDatabase<'ast> {
    /// Walk all enums in the schema.
    pub fn walk_enums(&self) -> impl Iterator<Item = EnumWalker<'ast, '_>> {
        self.ast()
            .iter_tops()
            .filter_map(|(top_id, _)| top_id.as_enum_id())
            .map(move |enum_id| Walker { db: self, id: enum_id })
    }

    /// Find a model by ID.
    pub(crate) fn walk_model(&self, model_id: ast::ModelId) -> ModelWalker<'ast, '_> {
        ModelWalker {
            model_id,
            db: self,
            model_attributes: &self.types.model_attributes[&model_id],
        }
    }

    /// Walk all the models in the schema.
    pub fn walk_models(&self) -> impl Iterator<Item = ModelWalker<'ast, '_>> + '_ {
        self.ast()
            .iter_tops()
            .filter_map(|(top_id, _)| top_id.as_model_id())
            .map(move |model_id| self.walk_model(model_id))
    }

    /// Walk a specific composite type by ID.
    pub fn walk_composite_type(&self, ctid: ast::CompositeTypeId) -> CompositeTypeWalker<'ast, '_> {
        CompositeTypeWalker { ctid, db: self }
    }

    /// Walk all the composite types in the schema.
    pub fn walk_composite_types(&self) -> impl Iterator<Item = CompositeTypeWalker<'ast, '_>> + '_ {
        self.ast()
            .iter_tops()
            .filter_map(|(top_id, _)| top_id.as_composite_type_id())
            .map(move |ctid| CompositeTypeWalker { ctid, db: self })
    }

    /// Walk all the relations in the schema. A relation may be defined by one or two fields; in
    /// both cases, it is still a single relation.
    pub fn walk_relations(&self) -> impl Iterator<Item = RelationWalker<'ast, '_>> + '_ {
        (0..self.relations.relations_storage.len()).map(move |relation_id| RelationWalker { db: self, relation_id })
    }

    /// Iterate all complete relations that are not many to many and are
    /// correctly defined from both sides.
    #[track_caller]
    pub fn walk_complete_inline_relations(&self) -> impl Iterator<Item = CompleteInlineRelationWalker<'ast, '_>> + '_ {
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
                    })
            })
    }
}
