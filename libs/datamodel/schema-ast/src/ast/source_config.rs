use super::{Comment, ConfigBlockProperty, Identifier, Span, WithDocumentation, WithIdentifier, WithSpan};

/// A source block declaration.
#[derive(Debug, Clone)]
pub struct SourceConfig {
    /// Name of this source.
    pub name: Identifier,
    /// Top-level configuration properties for this source.
    pub properties: Vec<ConfigBlockProperty>,
    /// The comments for this source block.
    pub documentation: Option<Comment>,
    /// The location of this source block in the text representation.
    pub span: Span,
}

impl WithIdentifier for SourceConfig {
    fn identifier(&self) -> &Identifier {
        &self.name
    }
}

impl WithSpan for SourceConfig {
    fn span(&self) -> &Span {
        &self.span
    }
}

impl WithDocumentation for SourceConfig {
    fn documentation(&self) -> &Option<Comment> {
        &self.documentation
    }

    fn is_commented_out(&self) -> bool {
        false
    }
}
