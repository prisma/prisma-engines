mod argument;
mod comment;
mod directive;
mod r#enum;
mod expression;
mod field;
mod generator_config;
mod identifier;
mod model;
mod source_config;
mod span;
mod top;
mod traits;

pub mod parser;
pub mod reformat;
pub mod renderer;

pub use argument::*;
pub use comment::*;
pub use directive::*;
pub use expression::*;
pub use field::*;
pub use generator_config::*;
pub use identifier::*;
pub use model::*;
pub use r#enum::*;
pub use source_config::*;
pub use span::*;
pub use top::*;
pub use traits::*;

/// AST representation of a prisma datamodel
///
/// This module is used internally to represent an AST. The AST's nodes can be used
/// during validation of a schema, especially when implementing custom directives.
///
/// The AST is not validated, also fields and directives are not resolved. Every node is
/// annotated with it's location in the text representation.
/// Basically, the AST is an object oriented representation of the datamodel's text.

/// A prisma schema.
/// Schema = Datamodel + Generators + Datasources
#[derive(Debug)]
pub struct SchemaAst {
    /// All models, enums, datasources, generators or type aliases
    pub tops: Vec<Top>,
}

impl SchemaAst {
    pub fn types(&self) -> Vec<&Field> {
        self.tops
            .iter()
            .filter_map(|top| match top {
                Top::Type(x) => Some(x),
                _ => None,
            })
            .collect()
    }

    pub fn enums(&self) -> Vec<&Enum> {
        self.tops
            .iter()
            .filter_map(|top| match top {
                Top::Enum(x) => Some(x),
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

    pub fn sources(&self) -> Vec<&SourceConfig> {
        self.tops
            .iter()
            .filter_map(|top| match top {
                Top::Source(x) => Some(x),
                _ => None,
            })
            .collect()
    }

    pub fn generators(&self) -> Vec<&GeneratorConfig> {
        self.tops
            .iter()
            .filter_map(|top| match top {
                Top::Generator(x) => Some(x),
                _ => None,
            })
            .collect()
    }
}
