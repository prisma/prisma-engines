use super::{Attribute, Comment, Identifier, Span};
use crate::ast::{Argument, SourceConfig};
use crate::diagnostics::Diagnostics;

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

    fn validate_attributes(&self) -> Diagnostics {
        let mut errors = Diagnostics::new();
        for attribute in self.attributes() {
            errors.push_opt_error(attribute.name.validate("Attribute").err());
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
    Attribute(&'a mut Attribute),
}
impl ArgumentContainer<'_> {
    pub fn name(&self) -> &str {
        match self {
            ArgumentContainer::SourceConfig(sc) => &sc.name.name,
            ArgumentContainer::Attribute(d) => &d.name.name,
        }
    }

    pub fn arguments(&mut self) -> &mut Vec<Argument> {
        match self {
            ArgumentContainer::SourceConfig(sc) => &mut sc.properties,
            ArgumentContainer::Attribute(d) => &mut d.arguments,
        }
    }

    pub fn set_arguments(&mut self, arguments: Vec<Argument>) {
        match self {
            ArgumentContainer::SourceConfig(sc) => sc.properties = arguments,
            ArgumentContainer::Attribute(d) => d.arguments = arguments,
        }
    }
}
