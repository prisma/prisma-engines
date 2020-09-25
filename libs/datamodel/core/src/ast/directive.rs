use super::*;

#[derive(Debug, Clone, PartialEq)]
pub struct Directive {
    pub name: Identifier,
    pub arguments: Vec<Argument>,
    /// The comments for this directive.
    pub documentation: Option<Comment>,
    pub span: Span,
    pub is_commented_out: bool,
}

impl Directive {
    pub fn new(name: &str, documentation: Option<Comment>, arguments: Vec<Argument>) -> Directive {
        Directive {
            name: Identifier::new(name),
            arguments,
            documentation,
            span: Span::empty(),
            is_commented_out: false,
        }
    }
}

impl WithDocumentation for Directive {
    fn documentation(&self) -> &Option<Comment> {
        &self.documentation
    }

    fn is_commented_out(&self) -> bool {
        self.is_commented_out
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
