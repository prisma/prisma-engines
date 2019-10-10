use super::{Comment, Directive, Identifier, Span};

pub trait WithSpan {
    fn span(&self) -> &Span;
}

pub trait WithName {
    fn name(&self) -> &str;
}

pub trait WithIdentifier {
    fn identifier(&self) -> &Identifier;
}

pub trait WithDirectives {
    fn directives(&self) -> &Vec<Directive>;
}

pub trait WithDocumentation {
    fn documentation(&self) -> &Option<Comment>;
}

// generic implementations

impl<T> WithName for T
where
    T: WithIdentifier,
{
    fn name(&self) -> &str {
        &self.identifier().name
    }
}
