//! This module contains the data structures and parsing function for the AST of a Prisma schema.
//! The responsibilities of the sub modules are as following:
//! * `parser`: Exposes the parse function that turns a String input into an AST.
//! * `reformat`: Exposes a Formatter for Prisma files. This is used e.g. by the VS Code Extension.
//! * `renderer`: Turns an AST into a Prisma Schema String.
mod argument;
mod attribute;
mod comment;
mod r#enum;
mod expression;
mod field;
mod generator_config;
mod helper;
mod identifier;
mod model;
mod parser;
mod renderer;
mod source_config;
mod span;
mod top;
mod traits;

pub mod reformat;

pub use argument::Argument;
pub use attribute::Attribute;
pub use comment::Comment;
pub use expression::Expression;
pub use field::{Field, FieldArity, FieldType};
pub use generator_config::GeneratorConfig;
pub use identifier::Identifier;
pub use r#enum::{Enum, EnumValue};
pub use source_config::SourceConfig;
pub use span::Span;
pub use top::Top;
pub use traits::{ArgumentContainer, WithAttributes, WithDocumentation, WithIdentifier, WithName, WithSpan};

pub(crate) use model::{FieldId, Model};
pub(crate) use parser::parse_schema;
pub(crate) use renderer::Renderer;

/// AST representation of a prisma schema.
///
/// This module is used internally to represent an AST. The AST's nodes can be used
/// during validation of a schema, especially when implementing custom attributes.
///
/// The AST is not validated, also fields and attributes are not resolved. Every node is
/// annotated with its location in the text representation.
/// Basically, the AST is an object oriented representation of the datamodel's text.
/// Schema = Datamodel + Generators + Datasources
#[derive(Debug, Clone, PartialEq)]
pub struct SchemaAst {
    /// All models, enums, datasources, generators or type aliases
    pub(super) tops: Vec<Top>,
}

impl SchemaAst {
    pub fn empty() -> Self {
        SchemaAst { tops: Vec::new() }
    }

    // Deprecated. Use ParserDatabase instead where possible.
    pub(crate) fn find_model(&self, model: &str) -> Option<&Model> {
        self.iter_models().find(|(_, m)| m.name.name == model).map(|(_, m)| m)
    }

    // Deprecated. Use ParserDatabase instead where possible.
    pub(crate) fn find_field(&self, model: &str, field: &str) -> Option<&Field> {
        self.find_model(model)?.fields.iter().find(|f| f.name.name == field)
    }

    pub(crate) fn iter_models(&self) -> impl Iterator<Item = (ModelId, &Model)> {
        self.iter_tops().filter_map(|(top_id, top)| match (top_id, top) {
            (TopId::Model(model_id), Top::Model(model)) => Some((model_id, model)),
            _ => None,
        })
    }

    pub(crate) fn iter_tops(&self) -> impl Iterator<Item = (TopId, &Top)> {
        self.tops.iter().enumerate().map(|(top_idx, top)| {
            let top_id = match top {
                Top::Enum(_) => TopId::Enum(EnumId(top_idx as u32)),
                Top::Model(_) => TopId::Model(ModelId(top_idx as u32)),
                Top::Source(_) => TopId::Source(SourceId(top_idx as u32)),
                Top::Generator(_) => TopId::Generator(GeneratorId(top_idx as u32)),
                Top::Type(_) => TopId::Alias(AliasId(top_idx as u32)),
            };

            (top_id, top)
        })
    }

    pub(crate) fn sources(&self) -> impl Iterator<Item = &SourceConfig> {
        self.tops.iter().filter_map(|top| top.as_source())
    }

    pub(crate) fn generators(&self) -> impl Iterator<Item = &GeneratorConfig> {
        self.tops.iter().filter_map(|top| top.as_generator())
    }
}

/// An opaque identifier for a model in a schema AST. Use the
/// `schema[model_id]` syntax to resolve the id to an `ast::Model`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct ModelId(u32);

impl std::ops::Index<ModelId> for SchemaAst {
    type Output = Model;

    fn index(&self, index: ModelId) -> &Self::Output {
        self.tops[index.0 as usize].as_model().unwrap()
    }
}

/// An opaque identifier for an enum in a schema AST.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct EnumId(u32);

impl std::ops::Index<EnumId> for SchemaAst {
    type Output = Enum;

    fn index(&self, index: EnumId) -> &Self::Output {
        self.tops[index.0 as usize].as_enum().unwrap()
    }
}

/// An opaque identifier for a type alias in a schema AST. Use the
/// `schema[alias_id]` syntax to resolve the id to an `ast::Field`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct AliasId(u32);

impl std::ops::Index<AliasId> for SchemaAst {
    type Output = Field;

    fn index(&self, index: AliasId) -> &Self::Output {
        self.tops[index.0 as usize].as_type_alias().unwrap()
    }
}

/// An opaque identifier for a generator block in a schema AST.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct GeneratorId(u32);

/// An opaque identifier for a datasource blèck in a schema AST.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct SourceId(u32);

/// An identifier for a top-level item in a schema AST. Use the `schema[top_id]`
/// syntax to resolve the id to an `ast::Top`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum TopId {
    Model(ModelId),
    Enum(EnumId),
    Alias(AliasId),
    Generator(GeneratorId),
    Source(SourceId),
}

impl TopId {
    pub(crate) fn as_model_id(&self) -> Option<ModelId> {
        match self {
            TopId::Model(model_id) => Some(*model_id),
            _ => None,
        }
    }
}

impl std::ops::Index<TopId> for SchemaAst {
    type Output = Top;

    fn index(&self, index: TopId) -> &Self::Output {
        let idx = match index {
            TopId::Model(ModelId(idx)) => idx,
            TopId::Enum(EnumId(idx)) => idx,
            TopId::Alias(AliasId(idx)) => idx,
            TopId::Generator(GeneratorId(idx)) => idx,
            TopId::Source(SourceId(idx)) => idx,
        };

        &self.tops[idx as usize]
    }
}
