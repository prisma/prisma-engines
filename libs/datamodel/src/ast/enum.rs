use super::*;

/// An enum declaration.
#[derive(Debug)]
pub struct Enum {
    /// The name of the enum.
    pub name: Identifier,
    /// The values of the enum.
    pub values: Vec<EnumValue>,
    /// The directives of this enum.
    pub directives: Vec<Directive>,
    /// The comments for this enum.
    pub documentation: Option<Comment>,
    /// The location of this enum in the text representation.
    pub span: Span,
}

impl WithIdentifier for Enum {
    fn identifier(&self) -> &Identifier {
        &self.name
    }
}

impl WithSpan for Enum {
    fn span(&self) -> &Span {
        &self.span
    }
}

impl WithDirectives for Enum {
    fn directives(&self) -> &Vec<Directive> {
        &self.directives
    }
}

impl WithDocumentation for Enum {
    fn documentation(&self) -> &Option<Comment> {
        &self.documentation
    }
}

/// An enum value definition.
#[derive(Debug)]
pub struct EnumValue {
    /// The name of the enum value.
    pub name: String,
    /// The location of this enum value in the text representation.
    pub span: Span,
}

impl WithName for EnumValue {
    fn name(&self) -> &str {
        &self.name
    }
}

impl WithSpan for EnumValue {
    fn span(&self) -> &Span {
        &self.span
    }
}
