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

pub use crate::types::RelationFieldId;
pub use composite_type::*;
pub use field::*;
pub use index::*;
pub use model::*;
pub use r#enum::*;
pub use relation::*;
pub use relation_field::*;
pub use scalar_field::*;

/// A generic walker. Only walkers intantiated with a concrete ID type (`I`) are useful.
#[derive(Clone, Copy)]
pub struct Walker<'db, I> {
    /// The parser database being traversed.
    pub db: &'db crate::ParserDatabase,
    /// The identifier of the focused element.
    pub id: I,
}

impl<'db, I> Walker<'db, I> {
    /// Traverse something else in the same schema.
    pub fn walk<J>(self, other: J) -> Walker<'db, J> {
        self.db.walk(other)
    }
}

impl<'db, I> PartialEq for Walker<'db, I>
where
    I: PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        self.id.eq(&other.id)
    }
}

impl crate::ParserDatabase {
    /// Find an enum by name.
    pub fn find_enum<'db>(&'db self, name: &str) -> Option<EnumWalker<'db>> {
        self.interner
            .lookup(name)
            .and_then(|name_id| self.names.tops.get(&name_id))
            .and_then(|top_id| top_id.as_enum_id())
            .map(|enum_id| self.walk(enum_id))
    }

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

    /// Walk all the models in the schema.
    pub fn walk_models(&self) -> impl Iterator<Item = ModelWalker<'_>> + '_ {
        self.ast()
            .iter_tops()
            .filter_map(|(top_id, _)| top_id.as_model_id())
            .map(move |model_id| self.walk(model_id))
            .filter(|m| !m.ast_model().is_view())
    }

    /// Walk all the views in the schema.
    pub fn walk_views(&self) -> impl Iterator<Item = ModelWalker<'_>> + '_ {
        self.ast()
            .iter_tops()
            .filter_map(|(top_id, _)| top_id.as_model_id())
            .map(move |model_id| self.walk(model_id))
            .filter(|m| m.ast_model().is_view())
    }

    /// Walk all the composite types in the schema.
    pub fn walk_composite_types(&self) -> impl Iterator<Item = CompositeTypeWalker<'_>> + '_ {
        self.ast()
            .iter_tops()
            .filter_map(|(top_id, _)| top_id.as_composite_type_id())
            .map(|id| self.walk(id))
    }

    /// Walk all scalar field defaults with a function not part of the common ones.
    pub fn walk_scalar_field_defaults_with_unknown_function(&self) -> impl Iterator<Item = DefaultValueWalker<'_>> {
        self.types
            .unknown_function_defaults
            .iter()
            .map(|id| DefaultValueWalker {
                field_id: *id,
                db: self,
                default: self.types[*id].default.as_ref().unwrap(),
            })
    }

    /// Walk all the relations in the schema. A relation may be defined by one or two fields; in
    /// both cases, it is still a single relation.
    pub fn walk_relations(&self) -> impl ExactSizeIterator<Item = RelationWalker<'_>> + Clone + '_ {
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
            .filter(|(relation, _)| !relation.is_implicit_many_to_many())
            .filter_map(move |(relation, _)| {
                relation
                    .as_complete_fields()
                    .map(|(field_a, field_b)| CompleteInlineRelationWalker {
                        side_a: field_a,
                        side_b: field_b,
                        db: self,
                    })
            })
    }
}
