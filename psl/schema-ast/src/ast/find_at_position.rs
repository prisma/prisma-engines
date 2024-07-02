mod attribute;
mod composite_type;
mod datasource;
mod r#enum;
mod expression;
mod field;
mod generator;
mod model;
mod property;

pub use attribute::AttributePosition;
pub use composite_type::CompositeTypePosition;
pub use datasource::SourcePosition;
pub use expression::ExpressionPosition;
pub use field::FieldPosition;
pub use generator::GeneratorPosition;
pub use model::ModelPosition;
pub use property::PropertyPosition;
pub use r#enum::EnumPosition;

use crate::ast::{self, top_idx_to_top_id, traits::*};

impl ast::SchemaAst {
    /// Find the AST node at the given position (byte offset).
    pub fn find_at_position(&self, position: usize) -> SchemaPosition<'_> {
        self.find_top_at_position(position)
            .map(|top_id| match top_id {
                ast::TopId::Model(model_id) => {
                    SchemaPosition::Model(model_id, ModelPosition::new(&self[model_id], position))
                }
                ast::TopId::Enum(enum_id) => SchemaPosition::Enum(enum_id, EnumPosition::new(&self[enum_id], position)),
                ast::TopId::CompositeType(composite_type_id) => SchemaPosition::CompositeType(
                    composite_type_id,
                    CompositeTypePosition::new(&self[composite_type_id], position),
                ),
                ast::TopId::Source(source_id) => {
                    SchemaPosition::DataSource(source_id, SourcePosition::new(&self[source_id], position))
                }
                ast::TopId::Generator(generator_id) => {
                    SchemaPosition::Generator(generator_id, GeneratorPosition::new(&self[generator_id], position))
                }
            })
            // If no top matched, we're in between top-level items. This is normal and expected.
            .unwrap_or(SchemaPosition::TopLevel)
    }

    /// Do a binary search for the `Top` at the given byte offset.
    pub fn find_top_at_position(&self, position: usize) -> Option<ast::TopId> {
        use std::cmp::Ordering;

        let top_idx = self.tops.binary_search_by(|top| {
            let span = top.span();

            if span.start > position {
                Ordering::Greater
            } else if span.end < position {
                Ordering::Less
            } else {
                Ordering::Equal
            }
        });

        top_idx.map(|idx| top_idx_to_top_id(idx, &self.tops[idx])).ok()
    }
}

/// A cursor position in a schema.
#[derive(Debug)]
pub enum SchemaPosition<'ast> {
    /// In-between top-level items
    TopLevel,
    /// In a model
    Model(ast::ModelId, ModelPosition<'ast>),
    /// In an enum
    Enum(ast::EnumId, EnumPosition<'ast>),
    /// In a composite type
    CompositeType(ast::CompositeTypeId, CompositeTypePosition<'ast>),
    /// In a datasource
    DataSource(ast::SourceId, SourcePosition<'ast>),
    /// In a generator
    Generator(ast::GeneratorId, GeneratorPosition<'ast>),
}
