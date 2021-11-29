use crate::helper::pretty_print;
use crate::Span;
use thiserror::Error;

/// Enum for different warnings which can happen during parsing or validation.
///
/// For fancy printing, please use the `pretty_print_error` function.
#[derive(Debug, Error, Clone, PartialEq)]
pub enum DatamodelWarning {
    #[error("Preview feature \"{}\" is deprecated. The functionality can be used without specifying it as a preview feature.", preview_feature)]
    DeprecatedPreviewFeature { preview_feature: String, span: Span },
}

impl DatamodelWarning {
    pub fn new_deprecated_preview_feature_warning(preview_feature: &str, span: Span) -> DatamodelWarning {
        DatamodelWarning::DeprecatedPreviewFeature {
            preview_feature: String::from(preview_feature),
            span,
        }
    }

    pub fn span(&self) -> Span {
        match self {
            DatamodelWarning::DeprecatedPreviewFeature { span, .. } => *span,
        }
    }

    pub fn description(&self) -> String {
        self.to_string()
    }

    pub fn pretty_print(&self, f: &mut dyn std::io::Write, file_name: &str, text: &str) -> std::io::Result<()> {
        pretty_print(f, file_name, text, self.span(), self.description().as_str())
    }
}
