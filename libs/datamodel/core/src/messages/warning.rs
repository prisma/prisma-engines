use crate::ast::Span;
use thiserror::Error;

// No format for this file, on purpose.
// Line breaks make the declarations very hard to read.
#[rustfmt::skip]
/// Enum for different warnings which can happen during parsing or validation.
#[derive(Debug, Error, Clone, PartialEq)]
pub enum DatamodelWarning {
  #[error("The preview feature \"{}\" is deprecated.", preview_feature)]
  PreviewFeatureDeprecationWarning { preview_feature: String, span: Span },
}

impl DatamodelWarning {
    pub fn new_preview_feature_deprecation_warning(preview_feature: &str, span: Span) -> DatamodelWarning {
        DatamodelWarning::PreviewFeatureDeprecationWarning {
            preview_feature: String::from(preview_feature),
            span,
        }
    }

    pub fn span(&self) -> Span {
        match self {
            DatamodelWarning::PreviewFeatureDeprecationWarning { span, .. } => *span,
        }
    }
}
