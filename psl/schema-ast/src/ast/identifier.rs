use super::{Span, WithSpan};
use diagnostics::FileId;

/// An identifier.
#[derive(Debug, Clone, PartialEq)]
pub struct Identifier {
    /// The identifier contents.
    pub name: String,
    /// The span of the AST node.
    pub span: Span,
}

impl Identifier {
    pub(crate) fn new<T: pest::RuleType>(pair: pest::iterators::Pair<'_, T>, file_id: FileId) -> Self {
        Identifier {
            name: pair.as_str().to_owned(),
            span: (file_id, pair.as_span()).into(),
        }
    }
}

impl WithSpan for Identifier {
    fn span(&self) -> Span {
        self.span
    }
}
