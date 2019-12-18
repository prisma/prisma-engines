use super::*;

/// A Generator block declaration.
#[derive(Debug, Clone, PartialEq)]
pub struct GeneratorConfig {
    /// Name of this generator.
    pub name: Identifier,
    /// Top-level configuration properties for this generator.
    pub properties: Vec<Argument>,
    /// The comments for this generator block.
    pub documentation: Option<Comment>,
    /// The location of this generator block in the text representation.
    pub span: Span,
}

impl WithIdentifier for GeneratorConfig {
    fn identifier(&self) -> &Identifier {
        &self.name
    }
}

impl WithSpan for GeneratorConfig {
    fn span(&self) -> &Span {
        &self.span
    }
}

impl WithDocumentation for GeneratorConfig {
    fn documentation(&self) -> &Option<Comment> {
        &self.documentation
    }
}
