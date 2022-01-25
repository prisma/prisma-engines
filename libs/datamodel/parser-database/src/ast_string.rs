use schema_ast::ast;

/// A maybe-allocated AST string.
#[derive(Debug, Clone)]
pub(crate) struct AstString {
    pub(crate) span: ast::Span,
    /// The unescaped string if applicable.
    pub(crate) unescaped: Option<String>,
}

impl AstString {
    pub(crate) fn from_literal(v: String, span: ast::Span) -> Self {
        AstString {
            span,
            unescaped: Some(v),
        }
    }
}
