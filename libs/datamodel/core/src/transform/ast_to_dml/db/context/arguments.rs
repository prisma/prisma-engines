use crate::ast;
use crate::diagnostics::{DatamodelError, Diagnostics};
use crate::transform::helpers::ValueValidator;
use std::collections::HashMap;

/// Represents a list of arguments.
#[derive(Debug)]
pub(crate) struct Arguments<'a> {
    attribute: Option<&'a ast::Attribute>,
    args: HashMap<&'a str, &'a ast::Argument>, // the _remaining_ arguments
}

impl Default for Arguments<'_> {
    fn default() -> Self {
        Arguments {
            attribute: None,
            args: Default::default(),
        }
    }
}

impl<'a> Arguments<'a> {
    /// Starts validating the arguments for an attribute, checking for duplicate arguments in the process.
    pub(super) fn set_attribute(&mut self, attribute: &'a ast::Attribute) -> Result<(), Diagnostics> {
        let arguments = &attribute.arguments;
        self.attribute = Some(attribute);
        self.args.clear();
        self.args.reserve(arguments.len());
        let mut errors = Diagnostics::new();
        let mut unnamed_arguments = Vec::new();

        for arg in arguments {
            if let Some(existing_argument) = self.args.insert(arg.name.name.as_str(), arg) {
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
                self.span(),
            ))
        }

        errors.to_result()
    }

    pub(crate) fn attribute(&self) -> &'a ast::Attribute {
        self.attribute.unwrap()
    }

    /// Call this at the end of validation. It will report errors for each argument that was not used by the validators.
    pub(crate) fn check_for_unused_arguments(&self, errors: &mut Diagnostics) {
        for arg in self.args.values() {
            errors.push_error(DatamodelError::new_unused_argument_error(&arg.name.name, arg.span));
        }
    }

    /// Gets the span of all arguments wrapped by this instance.
    pub(crate) fn span(&self) -> ast::Span {
        self.attribute().span
    }

    pub(crate) fn optional_arg(&mut self, name: &str) -> Option<ValueValidator<'a>> {
        self.args.remove(name).map(|arg| ValueValidator::new(&arg.value))
    }

    /// True if argument with the given key is defined.
    pub(crate) fn has_arg(&self, name: &str) -> bool {
        self.args.contains_key(name)
    }

    /// Gets the arg with the given name, or if it is not found, the first unnamed argument.
    ///
    /// Use this to implement unnamed argument behavior.
    pub(crate) fn default_arg(&mut self, name: &str) -> Result<ValueValidator<'a>, DatamodelError> {
        match (self.args.remove(name), self.args.remove("")) {
            (Some(arg), None) => Ok(ValueValidator::new(&arg.value)),
            (None, Some(arg)) => Ok(ValueValidator::new(&arg.value)),
            (Some(arg), Some(_)) => Err(DatamodelError::new_duplicate_default_argument_error(name, arg.span)),
            (None, None) => Err(DatamodelError::new_argument_not_found_error(name, self.span())),
        }
    }

    pub(crate) fn new_attribute_validation_error(&self, message: &str) -> DatamodelError {
        DatamodelError::new_attribute_validation_error(message, self.attribute().name(), self.span())
    }

    pub(crate) fn optional_default_arg(&mut self, name: &str) -> Option<ValueValidator<'a>> {
        self.default_arg(name).ok()
    }
}
