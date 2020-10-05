use super::super::helpers::*;
use crate::ast;
use crate::dml;
use crate::error::DatamodelError;

/// Structs implementing this trait can be used to validate any
/// attribute and to apply the attribute's effect on the corresponding
/// datamodel object.
pub trait AttributeValidator<T> {
    /// Gets the attribute name.
    fn attribute_name(&self) -> &str;

    fn is_duplicate_definition_allowed(&self) -> bool {
        false
    }

    /// Validates an attribute and applies the attribute
    /// to the given object.
    fn validate_and_apply(&self, args: &mut Arguments, obj: &mut T) -> Result<(), DatamodelError>;

    /// Serializes the given attribute's arguments for rendering.
    fn serialize(&self, obj: &T, datamodel: &dml::Datamodel) -> Result<Vec<ast::Attribute>, DatamodelError>;

    /// Shorthand to construct an attribute validation error.
    fn new_attribute_validation_error(&self, msg: &str, span: ast::Span) -> Result<(), DatamodelError> {
        Err(DatamodelError::new_attribute_validation_error(
            msg,
            self.attribute_name(),
            span,
        ))
    }

    /// Shorthand to lift a generic parser error to an attribute validation error.
    fn wrap_in_attribute_validation_error(&self, err: &DatamodelError) -> DatamodelError {
        DatamodelError::new_attribute_validation_error(&format!("{}", err), self.attribute_name(), err.span())
    }
}
