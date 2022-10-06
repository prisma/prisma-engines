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
mod relation;
mod relation_field;
mod scalar_field;

pub use composite_type::*;
pub use field::*;
pub use index::*;
pub use model::*;
pub use r#enum::*;
pub use relation::*;
pub use relation_field::*;
pub use scalar_field::*;

use crate::{ast, ParserDatabase};

/// A generic walker. Only walkers intantiated with a concrete ID type (`I`) are useful.
#[derive(Clone, Copy)]
pub struct Walker<'db, I> {
    /// The parser database being traversed.
    pub db: &'db crate::ParserDatabase,
    /// The identifier of the focused element.
    pub id: I,
}

impl ParserDatabase {
    /// Traverse a schema element by id.
    pub fn walk<I>(&self, id: I) -> Walker<'_, I> {
        Walker { db: self, id }
    }

    /// Walk all enums in the schema.
    pub fn walk_enums(&self) -> impl Iterator<Item = EnumWalker<'_>> {
        self.ast()
            .iter_tops()
            .filter_map(|(top_id, _)| top_id.as_enum_id())
            .map(move |enum_id| Walker { db: self, id: enum_id })
    }

    /// Find a model by ID.
    pub(crate) fn walk_model(&self, model_id: ast::ModelId) -> ModelWalker<'_> {
        self.walk(model_id)
    }

    /// Find an enum by ID.
    pub fn walk_enum(&self, enum_id: ast::EnumId) -> EnumWalker<'_> {
        Walker { db: self, id: enum_id }
    }

    /// Walk all the models in the schema.
    pub fn walk_models(&self) -> impl Iterator<Item = ModelWalker<'_>> + '_ {
        self.ast()
            .iter_tops()
            .filter_map(|(top_id, _)| top_id.as_model_id())
            .map(move |model_id| self.walk_model(model_id))
    }

    /// Walk a specific composite type by ID.
    pub fn walk_composite_type(&self, ctid: ast::CompositeTypeId) -> CompositeTypeWalker<'_> {
        CompositeTypeWalker { ctid, db: self }
    }

    /// Walk all the composite types in the schema.
    pub fn walk_composite_types(&self) -> impl Iterator<Item = CompositeTypeWalker<'_>> + '_ {
        self.ast()
            .iter_tops()
            .filter_map(|(top_id, _)| top_id.as_composite_type_id())
            .map(move |ctid| CompositeTypeWalker { ctid, db: self })
    }

    /// Walk all scalar field defaults with a function not part of the common ones.
    pub fn walk_scalar_field_defaults_with_unknown_function(&self) -> impl Iterator<Item = DefaultValueWalker<'_>> {
        self.types
            .unknown_function_defaults
            .iter()
            .map(|(model_id, field_id)| DefaultValueWalker {
                model_id: *model_id,
                field_id: *field_id,
                db: self,
                default: self.types.scalar_fields[&(*model_id, *field_id)]
                    .default
                    .as_ref()
                    .unwrap(),
            })
    }

    /// Walk all the relations in the schema. A relation may be defined by one or two fields; in
    /// both cases, it is still a single relation.
    pub fn walk_relations(&self) -> impl Iterator<Item = RelationWalker<'_>> + '_ {
        self.relations.iter().map(move |relation_id| Walker {
            db: self,
            id: relation_id,
        })
    }

    /// Iterate all complete relations that are not many to many and are
    /// correctly defined from both sides.
    #[track_caller]
    pub fn walk_complete_inline_relations(&self) -> impl Iterator<Item = CompleteInlineRelationWalker<'_>> + '_ {
        self.relations
            .iter_relations()
            .filter(|(_, _, relation)| !relation.is_implicit_many_to_many())
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
