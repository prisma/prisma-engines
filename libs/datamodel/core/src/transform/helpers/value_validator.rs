use super::env_function::EnvFunction;
use crate::diagnostics::DatamodelError;
use crate::{
    ast::{self, Expression, Span},
    configuration::StringFromEnvVar,
};
use crate::{DefaultValue, ValueGenerator};
use bigdecimal::BigDecimal;
use chrono::{DateTime, FixedOffset};
use dml::relation_info::ReferentialAction;
use dml::scalars::ScalarType;
use prisma_value::PrismaValue;
use std::error;

/// Wraps a value and provides convenience methods for
/// parsing it.
#[derive(Debug)]
pub struct ValueValidator<'a> {
    value: &'a ast::Expression,
}

impl<'a> ValueValidator<'a> {
    /// Creates a new instance by wrapping a value.
    ///
    /// If the value is a function expression, it is evaluated
    /// recursively.
    pub fn new(value: &'a ast::Expression) -> ValueValidator<'a> {
        ValueValidator { value }
    }

    /// Creates a new type mismatch error for the
    /// value wrapped by this instance.
    fn construct_type_mismatch_error(&self, expected_type: &str) -> DatamodelError {
        let description = String::from(self.value.describe_value_type());
        DatamodelError::new_type_mismatch_error(expected_type, &description, &self.raw(), self.span())
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
                format!("{}", err).as_ref(),
                &self.raw(),
                self.span(),
            )),
        }
    }

    /// Attempts to parse the wrapped value
    /// to a given prisma type.
    pub fn as_type(&self, scalar_type: ScalarType) -> Result<PrismaValue, DatamodelError> {
        match scalar_type {
            ScalarType::Int => self.as_int().map(PrismaValue::Int),
            ScalarType::Float => self.as_float().map(PrismaValue::Float),
            ScalarType::Boolean => self.as_bool().map(PrismaValue::Boolean),
            ScalarType::DateTime => self.as_date_time().map(PrismaValue::DateTime),
            ScalarType::String => self.as_str().map(String::from).map(PrismaValue::String),
            ScalarType::Json => self.as_str().map(String::from).map(PrismaValue::String),
            ScalarType::Bytes => self.as_str().and_then(|s| {
                prisma_value::decode_bytes(s).map(PrismaValue::Bytes).map_err(|_| {
                    DatamodelError::new_validation_error(&format!("Invalid base64 string '{}'.", s), self.span())
                })
            }),

            ScalarType::Decimal => self.as_float().map(PrismaValue::Float),
            ScalarType::BigInt => self.as_int().map(PrismaValue::BigInt),
        }
    }

    /// Accesses the raw string representation
    /// of the wrapped value.
    pub fn raw(&self) -> String {
        self.value.to_string()
    }

    /// Accesses the span of the wrapped value.
    pub fn span(&self) -> ast::Span {
        self.value.span()
    }

    /// Tries to convert the wrapped value to a Prisma String.
    pub fn as_str(&self) -> Result<&'a str, DatamodelError> {
        match &self.value {
            ast::Expression::StringValue(value, _) => Ok(value),
            _ => Err(self.construct_type_mismatch_error("String")),
        }
    }

    /// returns a (Some(a), b) if the string was deducted from an env var
    pub fn as_str_from_env(&self) -> Result<StringFromEnvVar, DatamodelError> {
        match &self.value {
            ast::Expression::Function(name, _, _) if name == "env" => {
                let env_function = self.as_env_function()?;
                Ok(StringFromEnvVar::new_from_env_var(env_function.var_name().to_owned()))
            }
            ast::Expression::StringValue(value, _) => Ok(StringFromEnvVar::new_literal(value.clone())),
            _ => Err(self.construct_type_mismatch_error("String")),
        }
    }

    pub(crate) fn as_env_function(&self) -> Result<EnvFunction, DatamodelError> {
        EnvFunction::from_ast(self.value)
    }

    /// returns true if this argument is derived from an env() function
    pub(crate) fn is_from_env(&self) -> bool {
        self.value.is_env_expression()
    }

    /// Tries to convert the wrapped value to a Prisma Integer.
    pub(crate) fn as_int(&self) -> Result<i64, DatamodelError> {
        match &self.value {
            ast::Expression::NumericValue(value, _) => self.wrap_error_from_result(value.parse::<i64>(), "numeric"),
            _ => Err(self.construct_type_mismatch_error("numeric")),
        }
    }

    /// Tries to convert the wrapped value to a Prisma Float.
    pub(crate) fn as_float(&self) -> Result<BigDecimal, DatamodelError> {
        match &self.value {
            ast::Expression::StringValue(value, _) => {
                self.wrap_error_from_result(value.parse::<BigDecimal>(), "numeric")
            }
            ast::Expression::NumericValue(value, _) => {
                self.wrap_error_from_result(value.parse::<BigDecimal>(), "numeric")
            }
            _ => Err(self.construct_type_mismatch_error("numeric")),
        }
    }

    /// Tries to convert the wrapped value to a Prisma Boolean.
    pub fn as_bool(&self) -> Result<bool, DatamodelError> {
        match &self.value {
            ast::Expression::BooleanValue(value, _) => self.wrap_error_from_result(value.parse::<bool>(), "boolean"),
            // this case is just here because `as_bool_from_env` passes a StringValue
            ast::Expression::StringValue(value, _) => self.wrap_error_from_result(value.parse::<bool>(), "boolean"),
            _ => Err(self.construct_type_mismatch_error("boolean")),
        }
    }

    /// Tries to convert the wrapped value to a Prisma DateTime.
    pub fn as_date_time(&self) -> Result<DateTime<FixedOffset>, DatamodelError> {
        match &self.value {
            ast::Expression::StringValue(value, _) => {
                self.wrap_error_from_result(DateTime::parse_from_rfc3339(value), "datetime")
            }
            _ => Err(self.construct_type_mismatch_error("dateTime")),
        }
    }

    /// Unwraps the value as an array of constants.
    pub fn as_constant_array(&self) -> Result<Vec<&'a str>, DatamodelError> {
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
    pub fn as_constant_literal(&self) -> Result<&'a str, DatamodelError> {
        match &self.value {
            ast::Expression::ConstantValue(value, _) => Ok(value),
            ast::Expression::BooleanValue(value, _) => Ok(value),
            _ => Err(self.construct_type_mismatch_error("constant literal")),
        }
    }

    /// Unwraps the wrapped value as a referential action.
    pub fn as_referential_action(&self) -> Result<ReferentialAction, DatamodelError> {
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

    pub fn as_default_value_for_scalar_type(&self, scalar_type: ScalarType) -> Result<DefaultValue, DatamodelError> {
        match &self.value {
            ast::Expression::Function(name, args, _) => {
                let prisma_args = match args.as_slice() {
                    [Expression::StringValue(_, _)] => {
                        let x = ValueValidator::new(args.first().unwrap()).as_type(ScalarType::String)?;
                        vec![x]
                    }
                    [] => vec![],
                    _ => return Err(DatamodelError::new_validation_error(&format!("DefaultValue function parsing failed. The function arg should only be empty or a single String. Got: `{:?}`. You can read about the available functions here: https://pris.ly/d/attribute-functions", args), self.span())),
                };
                let generator = self.get_value_generator(name, prisma_args)?;

                generator
                    .check_compatibility_with_scalar_type(scalar_type)
                    .map_err(|err_msg| DatamodelError::new_functional_evaluation_error(&err_msg, self.span()))?;

                Ok(DefaultValue::Expression(generator))
            }
            _ => {
                let x = ValueValidator::new(self.value).as_type(scalar_type)?;
                Ok(DefaultValue::Single(x))
            }
        }
    }

    /// Try to interpret the expression as a string literal.
    pub fn as_string_literal(&self) -> Option<(&str, Span)> {
        self.value.as_string_value()
    }

    pub fn as_value_generator(&self) -> Result<ValueGenerator, DatamodelError> {
        match &self.value {
            ast::Expression::Function(name, args, _) => {
                let prisma_args = match args.as_slice() {
                    [Expression::StringValue(_, _)] => {
                        let x = ValueValidator::new(args.first().unwrap()).as_type(ScalarType::String)?;
                        vec![x]
                    }
                    [] => vec![],
                    _ => return Err(self.construct_type_mismatch_error("String or empty")),
                };
                self.get_value_generator(name, prisma_args)
            }
            _ => Err(self.construct_type_mismatch_error("function")),
        }
    }

    fn get_value_generator(&self, name: &str, args: Vec<PrismaValue>) -> Result<ValueGenerator, DatamodelError> {
        ValueGenerator::new(name.to_string(), args)
            .map_err(|err_msg| DatamodelError::new_functional_evaluation_error(&err_msg, self.span()))
    }
}

pub(crate) trait ValueListValidator {
    fn to_str_vec(&self) -> Result<Vec<String>, DatamodelError>;
    fn to_string_from_env_var_vec(&self) -> Result<Vec<StringFromEnvVar>, DatamodelError>;
    fn to_literal_vec(&self) -> Result<Vec<String>, DatamodelError>;
}

impl ValueListValidator for Vec<ValueValidator<'_>> {
    fn to_string_from_env_var_vec(&self) -> Result<Vec<StringFromEnvVar>, DatamodelError> {
        self.iter().map(|val| val.as_str_from_env()).collect()
    }

    fn to_str_vec(&self) -> Result<Vec<String>, DatamodelError> {
        self.iter().map(|val| Ok(val.as_str()?.to_owned())).collect()
    }

    fn to_literal_vec(&self) -> Result<Vec<String>, DatamodelError> {
        self.iter()
            .map(|val| val.as_constant_literal().map(String::from))
            .collect()
    }
}
