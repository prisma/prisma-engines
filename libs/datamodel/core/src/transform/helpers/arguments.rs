use super::ValueValidator;
use crate::ast;
use crate::diagnostics::{DatamodelError, Diagnostics};
use std::collections::HashMap;

/// Represents a list of arguments.
#[derive(Debug)]
pub struct Arguments<'a> {
    args: HashMap<&'a str, &'a ast::Argument>, // the _remaining_ arguments
    span: ast::Span,
}

impl<'a> Arguments<'a> {
    /// Creates a new instance for an attribute, checking for duplicate arguments in the process.
    pub fn new(attribute: &'a ast::Attribute) -> Result<Arguments<'a>, Diagnostics> {
        let arguments = &attribute.arguments;
        let span = attribute.span;
        let mut remaining_arguments = HashMap::with_capacity(arguments.len()); // validation will succeed more often than not
        let mut errors = Diagnostics::new();
        let mut unnamed_arguments = Vec::new();

        for arg in arguments {
            if let Some(existing_argument) = remaining_arguments.insert(arg.name.name.as_str(), arg) {
                if arg.is_unnamed() {
                    if unnamed_arguments.is_empty() {
                        unnamed_arguments.push(existing_argument.value.render_to_string())
                    }

                    unnamed_arguments.push(arg.value.render_to_string())
                } else {
                    errors.push_error(DatamodelError::new_duplicate_argument_error(&arg.name.name, arg.span));
                }
            }
        }

        if !unnamed_arguments.is_empty() {
            errors.push_error(DatamodelError::new_attribute_validation_error(
                &format!("You provided multiple unnamed arguments. This is not possible. Did you forget the brackets? Did you mean `[{}]`?", unnamed_arguments.join(", ")),
                attribute.name.name.as_str(),
                span
            ))
        }

        errors.make_result()?;

        Ok(Arguments {
            args: remaining_arguments,
            span,
        })
    }

    /// Call this at the end of validation. It will report errors for each argument that was not used by the validators.
    pub(crate) fn check_for_unused_arguments(&self, errors: &mut Diagnostics) {
        for arg in self.args.values() {
            errors.push_error(DatamodelError::new_unused_argument_error(&arg.name.name, arg.span));
        }
    }

    /// Gets the span of all arguments wrapped by this instance.
    pub(crate) fn span(&self) -> ast::Span {
        self.span
    }

    /// Gets the arg with the given name.
    pub(crate) fn arg(&mut self, name: &str) -> Result<ValueValidator, DatamodelError> {
        self.optional_arg(name)
            .ok_or_else(|| DatamodelError::new_argument_not_found_error(name, self.span))
    }

    pub(crate) fn optional_arg(&mut self, name: &str) -> Option<ValueValidator> {
        self.args.remove(name).map(|arg| ValueValidator::new(&arg.value))
    }

    /// Gets the arg with the given name, or if it is not found, the first unnamed argument.
    ///
    /// Use this to implement unnamed argument behavior.
    pub fn default_arg(&mut self, name: &str) -> Result<ValueValidator, DatamodelError> {
        match (self.args.remove(name), self.args.remove("")) {
            (Some(arg), None) => Ok(ValueValidator::new(&arg.value)),
            (None, Some(arg)) => Ok(ValueValidator::new(&arg.value)),
            (Some(arg), Some(_)) => Err(DatamodelError::new_duplicate_default_argument_error(&name, arg.span)),
            (None, None) => Err(DatamodelError::new_argument_not_found_error(name, self.span)),
        }
    }

    /// Optional unnamed default arg

    pub fn optional_default_arg(&mut self, name: &str) -> Result<Option<ValueValidator>, DatamodelError> {
        match (self.args.remove(name), self.args.remove("")) {
            (Some(arg), None) => Ok(Some(ValueValidator::new(&arg.value))),
            (None, Some(arg)) => Ok(Some(ValueValidator::new(&arg.value))),
            (Some(arg), Some(_)) => Err(DatamodelError::new_duplicate_default_argument_error(&name, arg.span)),
            (None, None) => Ok(None),
        }
    }
}
