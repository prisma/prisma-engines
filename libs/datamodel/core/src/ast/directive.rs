use super::*;

#[derive(Debug, Clone, PartialEq)]
pub struct Directive {
    pub name: Identifier,
    pub arguments: Vec<Argument>,
    pub span: Span,
}

impl Directive {
    pub fn new(name: &str, arguments: Vec<Argument>) -> Directive {
        Directive {
            name: Identifier::new(name),
            arguments,
            span: Span::empty(),
        }
    }
}

impl WithIdentifier for Directive {
    fn identifier(&self) -> &Identifier {
        &self.name
    }
}

impl WithSpan for Directive {
    fn span(&self) -> &Span {
        &self.span
    }
}
