use super::*;

#[derive(Debug, Clone, PartialEq)]
pub struct Identifier {
    pub name: String,
    pub span: Span,
}

impl Identifier {
    pub fn new(name: &str) -> Identifier {
        Identifier {
            name: String::from(name),
            span: Span::empty(),
        }
    }
}

impl WithSpan for Identifier {
    fn span(&self) -> &Span {
        &self.span
    }
}
