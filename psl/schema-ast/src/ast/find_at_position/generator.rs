use super::{PropertyPosition, WithName};
use crate::ast::{self};

#[derive(Debug)]
pub enum GeneratorPosition<'ast> {
    /// In the general generator
    Generator,
    /// In the generator's name
    Name(&'ast str),
    /// In a property
    Property(&'ast str, PropertyPosition<'ast>),
}

impl<'ast> GeneratorPosition<'ast> {
    pub(crate) fn new(source: &'ast ast::GeneratorConfig, position: usize) -> Self {
        if source.name.span.contains(position) {
            return GeneratorPosition::Name(source.name());
        }

        for property in &source.properties {
            if property.span.contains(position) {
                return GeneratorPosition::Property(&property.name.name, PropertyPosition::new(property, position));
            }
        }

        GeneratorPosition::Generator
    }
}
