use super::*;

/// A type declaration.
#[derive(Debug, Clone, PartialEq)]
pub struct TypeDefinition {
    /// The name of the type.
    pub name: Identifier,
    /// The fields of the type.
    pub fields: Vec<Field>,
    /// The documentation for this type.
    pub documentation: Option<Comment>,
    /// The location of this type in the text representation.
    pub span: Span,
    /// Should this be commented out.
    pub commented_out: bool,
}

impl WithIdentifier for TypeDefinition {
    fn identifier(&self) -> &Identifier {
        &self.name
    }
}

impl WithSpan for TypeDefinition {
    fn span(&self) -> &Span {
        &self.span
    }
}

impl WithDocumentation for TypeDefinition {
    fn documentation(&self) -> &Option<Comment> {
        &self.documentation
    }

    fn is_commented_out(&self) -> bool {
        self.commented_out
    }
}
