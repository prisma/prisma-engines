use crate::{DatamodelError, Span};

pub struct NativeTypeErrorFactory {
    native_type: String,
    connector: String,
}

impl NativeTypeErrorFactory {
    pub fn new(native_type: String, connector: String) -> Self {
        NativeTypeErrorFactory { native_type, connector }
    }

    pub fn new_scale_larger_than_precision_error(self, span: Span) -> DatamodelError {
        DatamodelError::new(
            format!(
                "The scale must not be larger than the precision for the {} native type in {}.",
                self.native_type, self.connector
            ),
            span,
        )
    }

    pub fn new_incompatible_native_type_with_index(self, message: &str, span: Span) -> DatamodelError {
        DatamodelError::new(
            format!(
                "You cannot define an index on fields with native type `{}` of {}.{message}",
                self.native_type, self.connector
            ),
            span,
        )
    }

    pub fn new_incompatible_native_type_with_unique(self, message: &str, span: Span) -> DatamodelError {
        DatamodelError::new(
            format!(
                "Native type `{}` cannot be unique in {}.{message}",
                self.native_type, self.connector
            ),
            span,
        )
    }

    pub fn new_incompatible_native_type_with_id(self, message: &str, span: Span) -> DatamodelError {
        DatamodelError::new(
            format!(
                "Native type `{}` of {} cannot be used on a field that is `@id` or `@@id`.{message}",
                self.native_type, self.connector
            ),
            span,
        )
    }

    pub fn new_argument_m_out_of_range_error(self, message: &str, span: Span) -> DatamodelError {
        DatamodelError::new(
            format!(
                "Argument M is out of range for native type `{}` of {}: {message}",
                self.native_type, self.connector
            ),
            span,
        )
    }

    pub fn native_type_name_unknown(self, span: Span) -> DatamodelError {
        DatamodelError::new(
            format!(
                "Native type {} is not supported for {} connector.",
                self.native_type, self.connector
            ),
            span,
        )
    }
}
