use super::Args;
use crate::ast;
use crate::dml;
use crate::error::DatamodelError;

/// Structs implementing this trait can be used to validate any
/// directive and to apply the directive's effect on the corresponding
/// datamodel object.
pub trait DirectiveValidator<T> {
    /// Gets the directive name.
    fn directive_name(&self) -> &str;

    fn is_duplicate_definition_allowed(&self) -> bool {
        false
    }

    /// Validates a directive and applies the directive
    /// to the given object.
    fn validate_and_apply(&self, args: &mut Args, obj: &mut T) -> Result<(), DatamodelError>;

    /// Serializes the given directive's arguments for rendering.
    fn serialize(&self, obj: &T, datamodel: &dml::Datamodel) -> Result<Vec<ast::Directive>, DatamodelError>;

    /// Shorthand to construct an directive validation error.
    fn new_directive_validation_error(&self, msg: &str, span: ast::Span) -> Result<(), DatamodelError> {
        Err(DatamodelError::new_directive_validation_error(
            msg,
            self.directive_name(),
            span,
        ))
    }

    /// Shorthand to lift a generic parser error to an directive validation error.
    fn wrap_in_directive_validation_error(&self, err: &DatamodelError) -> DatamodelError {
        DatamodelError::new_directive_validation_error(&format!("{}", err), self.directive_name(), err.span())
    }
}
