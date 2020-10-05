use super::{Comment, Attribute, Identifier, Span};
use crate::ast::{Argument, SourceConfig};
use crate::error::ErrorCollection;

pub trait WithSpan {
    fn span(&self) -> &Span;
}

pub trait WithName {
    fn name(&self) -> &str;
}

pub trait WithIdentifier {
    fn identifier(&self) -> &Identifier;
}

pub trait WithAttributes {
    fn attributes(&self) -> &Vec<Attribute>;

    fn validate_attributes(&self) -> ErrorCollection {
        let mut errors = ErrorCollection::new();
        for attribute in self.attributes() {
            errors.push_opt(attribute.name.validate("Attribute").err());
        }
        errors
    }
}

pub trait WithDocumentation {
    fn documentation(&self) -> &Option<Comment>;

    fn is_commented_out(&self) -> bool;
}

impl<T> WithName for T
where
    T: WithIdentifier,
{
    fn name(&self) -> &str {
        &self.identifier().name
    }
}

pub enum ArgumentContainer<'a> {
    SourceConfig(&'a mut SourceConfig),
    Directive(&'a mut Attribute),
}
impl ArgumentContainer<'_> {
    pub fn name(&self) -> &str {
        match self {
            ArgumentContainer::SourceConfig(sc) => &sc.name.name,
            ArgumentContainer::Directive(d) => &d.name.name,
        }
    }

    pub fn arguments(&mut self) -> &mut Vec<Argument> {
        match self {
            ArgumentContainer::SourceConfig(sc) => &mut sc.properties,
            ArgumentContainer::Directive(d) => &mut d.arguments,
        }
    }

    pub fn set_arguments(&mut self, arguments: Vec<Argument>) {
        match self {
            ArgumentContainer::SourceConfig(sc) => sc.properties = arguments,
            ArgumentContainer::Directive(d) => d.arguments = arguments,
        }
    }
}
