use super::*;

/// A Encryptor block declaration.
#[derive(Debug, Clone, PartialEq)]
pub struct EncryptorConfig {
    /// Name of this encryptor.
    pub name: Identifier,
    /// Top-level configuration properties for this encryptor.
    pub properties: Vec<Argument>,
    /// The comments for this encryptor block.
    pub documentation: Option<Comment>,
    /// The location of this encryptor block in the text representation.
    pub span: Span,
}

impl WithIdentifier for EncryptorConfig {
    fn identifier(&self) -> &Identifier {
        &self.name
    }
}

impl WithSpan for EncryptorConfig {
    fn span(&self) -> &Span {
        &self.span
    }
}

impl WithDocumentation for EncryptorConfig {
    fn documentation(&self) -> &Option<Comment> {
        &self.documentation
    }

    fn is_commented_out(&self) -> bool {
        false
    }
}
