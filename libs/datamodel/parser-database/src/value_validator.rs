//! See [ValueValidator](./struct.ValueValidator.html).

use crate::{
    ast::{self, Expression, Span},
    relations::ReferentialAction,
    types::SortOrder,
};
use diagnostics::DatamodelError;
use std::error;

/// Wraps a value and provides convenience methods for
/// validating it.
#[derive(Debug)]
pub struct ValueValidator<'a> {
    /// The underlying AST expression.
    pub value: &'a ast::Expression,
}

impl<'a> ValueValidator<'a> {
    /// Creates a new instance by wrapping a value.
    pub fn new(value: &'a ast::Expression) -> ValueValidator<'a> {
        ValueValidator { value }
    }

    /// Creates a new type mismatch error for the
    /// value wrapped by this instance.
    fn construct_type_mismatch_error(&self, expected_type: &str) -> DatamodelError {
        let description = String::from(self.value.describe_value_type());
        DatamodelError::new_type_mismatch_error(expected_type, &description, &self.value.to_string(), self.span())
    }

    /// Creates a value parser error
    /// from some other parser error.
    fn wrap_error_from_result<T, E: error::Error>(
        &self,
        result: Result<T, E>,
        expected_type: &str,
    ) -> Result<T, DatamodelError> {
        match result {
            Ok(val) => Ok(val),
            Err(err) => Err(DatamodelError::new_value_parser_error(
                expected_type,
                &err.to_string(),
                &self.value.to_string(),
                self.value.span(),
            )),
        }
    }

    /// Accesses the span of the wrapped value.
    pub fn span(&self) -> ast::Span {
        self.value.span()
    }

    /// Tries to convert the wrapped value to a Prisma String.
    pub fn as_str(&self) -> Result<&'a str, DatamodelError> {
        self.as_string_literal()
            .map(|(s, _)| s)
            .ok_or_else(|| self.construct_type_mismatch_error("String"))
    }

    /// Tries to convert the wrapped value to a Prisma String.
    pub fn as_str_with_span(&self) -> Result<(&'a str, ast::Span), DatamodelError> {
        self.as_string_literal()
            .ok_or_else(|| self.construct_type_mismatch_error("String"))
    }

    /// Returns true if this argument is derived from an env() function
    pub fn is_from_env(&self) -> bool {
        self.value.is_env_expression()
    }

    /// Tries to convert the wrapped value to a Prisma Integer.
    pub(crate) fn as_int(&self) -> Result<i64, DatamodelError> {
        match &self.value {
            ast::Expression::NumericValue(value, _) => self.wrap_error_from_result(value.parse::<i64>(), "numeric"),
            _ => Err(self.construct_type_mismatch_error("numeric")),
        }
    }

    /// Unwraps the value as an array of constants.
    pub(crate) fn as_constant_array(&self) -> Result<Vec<&'a str>, DatamodelError> {
        if let ast::Expression::Array(values, _) = &self.value {
            values
                .iter()
                .map(|val| ValueValidator::new(val).as_constant_literal())
                .collect()
        } else {
            // Single values are accepted as array literals, for example in `@relation(fields: userId)`.
            Ok(vec![self.as_constant_literal()?])
        }
    }

    /// Unwraps the wrapped value as a constant literal.
    pub(crate) fn as_constant_literal(&self) -> Result<&'a str, DatamodelError> {
        match &self.value {
            ast::Expression::ConstantValue(value, _) => Ok(value),
            _ => Err(self.construct_type_mismatch_error("constant literal")),
        }
    }

    #[allow(clippy::type_complexity)]
    /// Unwraps the value as an array of constants.
    pub(crate) fn as_field_array_with_args(
        &self,
    ) -> Result<Vec<(&'a str, Option<SortOrder>, Option<u32>)>, DatamodelError> {
        if let ast::Expression::Array(values, _) = &self.value {
            values.iter().map(|val| ValueValidator::new(val).as_func()).collect()
        } else {
            // Single values are accepted as array literals, for example in `@relation(fields: userId)`.
            Ok(vec![self.as_func()?])
        }
    }

    fn as_func(&self) -> Result<(&'a str, Option<SortOrder>, Option<u32>), DatamodelError> {
        match &self.value {
            Expression::ConstantValue(field_name, _) => Ok((field_name, None, None)),
            Expression::Function(field_name, args, _) => {
                let (sort, length) = ValueValidator::field_args(&args.arguments)?;
                Ok((field_name, sort, length))
            }

            _ => Err(self.construct_type_mismatch_error("constant literal")),
        }
    }

    /// Unwraps the wrapped value as a constant literal.
    fn field_args(args: &[ast::Argument]) -> Result<(Option<SortOrder>, Option<u32>), DatamodelError> {
        let sort = args
            .iter()
            .find(|arg| arg.name.as_ref().map(|n| n.name.as_str()) == Some("sort"))
            .map(|arg| match arg.value.as_constant_value() {
                Some(("Asc", _)) => Ok(Some(SortOrder::Asc)),
                Some(("Desc", _)) => Ok(Some(SortOrder::Desc)),
                None => Ok(None),
                _ => Err(DatamodelError::ParserError {
                    expected_str: "Asc, Desc".to_owned(),
                    span: arg.span,
                }),
            })
            .transpose()?
            .flatten();

        let length = args
            .iter()
            .find(|arg| arg.name.as_ref().map(|n| n.name.as_str()) == Some("length"))
            .map(|arg| match &arg.value {
                Expression::NumericValue(s, _) => s.parse::<u32>().map_err(|_| DatamodelError::ParserError {
                    expected_str: "valid integer".to_string(),
                    span: arg.span,
                }),
                _ => Err(DatamodelError::ParserError {
                    expected_str: "valid integer".to_string(),
                    span: arg.span,
                }),
            })
            .transpose()?;

        Ok((sort, length))
    }

    /// Unwraps the wrapped value as a referential action.
    pub(crate) fn as_referential_action(&self) -> Result<ReferentialAction, DatamodelError> {
        match self.as_constant_literal()? {
            "Cascade" => Ok(ReferentialAction::Cascade),
            "Restrict" => Ok(ReferentialAction::Restrict),
            "NoAction" => Ok(ReferentialAction::NoAction),
            "SetNull" => Ok(ReferentialAction::SetNull),
            "SetDefault" => Ok(ReferentialAction::SetDefault),
            s => {
                let message = format!("Invalid referential action: `{}`", s);

                Err(DatamodelError::AttributeValidationError {
                    message,
                    attribute_name: String::from("relation"),
                    span: self.span(),
                })
            }
        }
    }

    /// Unwraps the wrapped value as a constant literal..
    pub fn as_array(&self) -> Vec<ValueValidator<'a>> {
        match &self.value {
            ast::Expression::Array(values, _) => {
                let mut validators: Vec<ValueValidator<'_>> = Vec::new();

                for value in values {
                    validators.push(ValueValidator::new(value));
                }

                validators
            }
            _ => vec![ValueValidator { value: self.value }],
        }
    }

    /// Try to interpret the expression as a string literal.
    pub fn as_string_literal(&self) -> Option<(&'a str, Span)> {
        self.value.as_string_value()
    }
}

/// ValueValidator for lists of values.
pub trait ValueListValidator {
    /// Try to unwrap the value as a list of strings.
    fn to_str_vec(&self) -> Result<Vec<String>, DatamodelError>;
}

impl ValueListValidator for Vec<ValueValidator<'_>> {
    fn to_str_vec(&self) -> Result<Vec<String>, DatamodelError> {
        self.iter().map(|val| Ok(val.as_str()?.to_owned())).collect()
    }
}
