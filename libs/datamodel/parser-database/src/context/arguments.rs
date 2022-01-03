use crate::ast::{self, WithName};
use crate::ValueValidator;
use crate::{DatamodelError, Diagnostics};
use std::collections::HashMap;

/// Represents a list of arguments.
#[derive(Debug, Default)]
pub(crate) struct Arguments<'a> {
    attribute: Option<&'a ast::Attribute>,
    args: HashMap<&'a str, &'a ast::Argument>, // the _remaining_ arguments
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
                        let rendered = schema_ast::renderer::Renderer::render_value_to_string(&existing_argument.value);
                        unnamed_arguments.push(rendered)
                    }

                    let rendered = schema_ast::renderer::Renderer::render_value_to_string(&arg.value);
                    unnamed_arguments.push(rendered)
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

        for arg in &attribute.empty_arguments {
            errors.push_error(DatamodelError::new_attribute_validation_error(
                &format!("The `{}` argument is missing a value.", arg.name.name),
                attribute.name(),
                arg.name.span,
            ))
        }

        if let Some(span) = attribute.trailing_comma {
            errors.push_error(DatamodelError::new_attribute_validation_error(
                "Trailing commas are not valid in attribute arguments, please remove the comma.",
                attribute.name(),
                span,
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
