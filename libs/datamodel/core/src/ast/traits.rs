use super::{Attribute, Comment, Identifier, Span};
use crate::diagnostics::Diagnostics;

pub(crate) trait WithSpan {
    fn span(&self) -> &Span;
}

pub(crate) trait WithName {
    fn name(&self) -> &str;
}

pub(crate) trait WithIdentifier {
    fn identifier(&self) -> &Identifier;
}

pub(crate) trait WithAttributes {
    fn attributes(&self) -> &Vec<Attribute>;

    fn validate_attributes(&self, diagnostics: &mut Diagnostics) {
        for attribute in self.attributes() {
            attribute.name.validate("Attribute", diagnostics);
        }
    }
}

pub(crate) trait WithDocumentation {
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
