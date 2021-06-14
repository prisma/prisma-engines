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

    pub fn find_model(&self, model: &str) -> Option<&Model> {
        self.models().into_iter().find(|m| m.name.name == model)
    }

    pub fn find_type_alias(&self, type_name: &str) -> Option<&Field> {
        self.types().into_iter().find(|t| t.name.name == type_name)
    }

    pub fn find_field(&self, model: &str, field: &str) -> Option<&Field> {
        self.find_model(model)?.fields.iter().find(|f| f.name.name == field)
    }

    pub(crate) fn iter_tops(&self) -> impl Iterator<Item = (TopId, &Top)> {
        self.tops
            .iter()
            .enumerate()
            .map(|(top_idx, top)| (TopId(top_idx as u32), top))
    }

    pub fn types(&self) -> Vec<&Field> {
        self.tops
            .iter()
            .filter_map(|top| match top {
                Top::Type(x) => Some(x),
                _ => None,
            })
            .collect()
    }

    pub fn models(&self) -> Vec<&Model> {
        self.tops
            .iter()
            .filter_map(|top| match top {
                Top::Model(x) => Some(x),
                _ => None,
            })
            .collect()
    }

    pub fn sources(&self) -> impl Iterator<Item = &SourceConfig> {
        self.tops.iter().filter_map(|top| match top {
            Top::Source(x) => Some(x),
            _ => None,
        })
    }

    pub fn generators(&self) -> impl Iterator<Item = &GeneratorConfig> {
        self.tops.iter().filter_map(|top| match top {
            Top::Generator(x) => Some(x),
            _ => None,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct TopId(u32);

impl std::ops::Index<TopId> for SchemaAst {
    type Output = Top;

    fn index(&self, index: TopId) -> &Self::Output {
        &self.tops[index.0 as usize]
    }
}
