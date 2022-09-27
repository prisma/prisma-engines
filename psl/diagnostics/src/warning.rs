use crate::Span;

/// A non-fatal warning emitted by the schema parser.
/// For fancy printing, please use the `pretty_print_error` function.
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct DatamodelWarning {
    message: String,
    span: Span,
}

impl DatamodelWarning {
    fn new(message: String, span: Span) -> DatamodelWarning {
        DatamodelWarning { message, span }
    }

    pub fn new_feature_deprecated(feature: &str, span: Span) -> DatamodelWarning {
        let message = format!(
            "Preview feature \"{feature}\" is deprecated. The functionality can be used without specifying it as a preview feature."
        );
        Self::new(message, span)
    }

    /// The user-facing warning message.
    pub fn message(&self) -> &str {
        &self.message
    }

    /// The source span the warning applies to.
    pub fn span(&self) -> Span {
        self.span
    }
}
