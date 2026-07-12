use super::{Comment, ConfigBlockProperty, Identifier, Span, WithDocumentation, WithIdentifier, WithSpan};

/// A source block declaration.
#[derive(Debug, Clone)]
pub struct SourceConfig {
    /// Name of this source.
    pub(crate) name: Identifier,
    /// Top-level configuration properties for this source.
    pub properties: Vec<ConfigBlockProperty>,
    /// The comments for this source block.
    pub(crate) documentation: Option<Comment>,
    /// The location of this source block in the text representation.
    pub span: Span,
    /// The span of the inner contents.
    pub inner_span: Span,
}

impl WithIdentifier for SourceConfig {
    fn identifier(&self) -> &Identifier {
        &self.name
    }
}

impl WithSpan for SourceConfig {
    fn span(&self) -> Span {
        self.span
    }
}

impl WithDocumentation for SourceConfig {
    fn documentation(&self) -> Option<&str> {
        self.documentation.as_ref().map(|doc| doc.text.as_str())
    }
}
