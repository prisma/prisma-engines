use super::*;

#[derive(Debug, Clone, PartialEq)]
pub struct Attribute {
    pub name: Identifier,
    pub arguments: Vec<Argument>,
    pub span: Span,
}

impl Attribute {
    pub fn new(name: &str, arguments: Vec<Argument>) -> Attribute {
        Attribute {
            name: Identifier::new(name),
            arguments,
            span: Span::empty(),
        }
    }
}

impl WithIdentifier for Attribute {
    fn identifier(&self) -> &Identifier {
        &self.name
    }
}

impl WithSpan for Attribute {
    fn span(&self) -> &Span {
        &self.span
    }
}
