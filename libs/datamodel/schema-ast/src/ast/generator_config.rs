use super::{Comment, Identifier, Span, WithDocumentation, WithIdentifier, WithSpan};
use crate::ast::config::ConfigBlockProperty;

/// A Generator block declaration.
#[derive(Debug, Clone)]
pub struct GeneratorConfig {
    /// Name of this generator.
    pub name: Identifier,
    /// Top-level configuration properties for this generator.
    pub properties: Vec<ConfigBlockProperty>,
    /// The comments for this generator block.
    pub(crate) documentation: Option<Comment>,
    /// The location of this generator block in the text representation.
    pub span: Span,
}

impl WithIdentifier for GeneratorConfig {
    fn identifier(&self) -> &Identifier {
        &self.name
    }
}

impl WithSpan for GeneratorConfig {
    fn span(&self) -> Span {
        self.span
    }
}

impl WithDocumentation for GeneratorConfig {
    fn documentation(&self) -> Option<&str> {
        self.documentation.as_ref().map(|doc| doc.text.as_str())
    }
}
