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
#[derive(Debug, Clone)]
pub struct SchemaAst {
    /// All models, enums, datasources, generators or type aliases
    pub tops: Vec<Top>,
}

impl SchemaAst {
    pub fn empty() -> Self {
        SchemaAst { tops: Vec::new() }
    }

    pub fn find_model(&self, model: &str) -> Option<&Model> {
        self.models().into_iter().find(|m| m.name.name == model)
    }

    pub fn find_model_mut(&mut self, model_name: &str) -> Option<&mut Model> {
        self.tops.iter_mut().find_map(|top| match top {
            Top::Model(model) if model.name.name == model_name => Some(model),
            _ => None,
        })
    }

    pub fn find_custom_type(&self, type_name: &str) -> Option<&Field> {
        self.types().into_iter().find(|t| t.name.name == type_name)
    }

    pub fn find_custom_type_mut(&mut self, type_name: &str) -> Option<&mut Field> {
        self.tops.iter_mut().find_map(|top| match top {
            Top::Type(custom_type) if custom_type.name.name == type_name => Some(custom_type),
            _ => None,
        })
    }

    pub fn find_enum(&self, enum_name: &str) -> Option<&Enum> {
        self.enums().into_iter().find(|e| e.name.name == enum_name)
    }

    pub fn find_enum_mut(&mut self, enum_name: &str) -> Option<&mut Enum> {
        self.tops.iter_mut().find_map(|top| match top {
            Top::Enum(r#enum) if r#enum.name.name == enum_name => Some(r#enum),
            _ => None,
        })
    }

    pub fn find_field(&self, model: &str, field: &str) -> Option<&Field> {
        self.find_model(model)?.fields.iter().find(|f| f.name.name == field)
    }

    pub fn find_field_mut(&mut self, model: &str, field: &str) -> Option<&mut Field> {
        self.find_model_mut(model).and_then(|model| {
            model
                .fields
                .iter_mut()
                .find(|model_field| model_field.name.name == field)
        })
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
