use super::{Argument, Identifier, Span, WithIdentifier, WithSpan};

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

    pub fn name(&self) -> &str {
        &self.name.name
    }

    pub fn is_index(&self) -> bool {
        matches!(self.name.name.as_str(), "index" | "unique")
    }

    pub fn is_id(&self) -> bool {
        matches!(self.name.name.as_str(), "id")
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
