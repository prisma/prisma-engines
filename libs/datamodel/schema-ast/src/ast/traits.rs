use super::{Attribute, Comment, Identifier, Span};

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
    fn attributes(&self) -> &[Attribute];
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
