use crate::error::DatamodelError;
use crate::{ast, DefaultValue, EnvFunction};
use crate::{dml, ValueGenerator};

use super::FromStrAndSpan;
use super::ScalarType;
use chrono::{DateTime, Utc};
use std::error;

/// Wraps a value and provides convenience methods for
/// parsing it.
#[derive(Debug)]
pub struct ValueValidator {
    value: ast::Expression,
}

impl ValueValidator {
    /// Creates a new instance by wrapping a value.
    ///
    /// If the value is a function expression, it is evaluated
    /// recursively.
    pub fn new(value: &ast::Expression) -> ValueValidator {
        ValueValidator { value: value.clone() }
    }

    /// Creates a new type mismatch error for the
    /// value wrapped by this instance.
    fn construct_type_mismatch_error(&self, expected_type: &str) -> DatamodelError {
        let description = String::from(ast::describe_value_type(&self.value));
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
                err.description(),
                &self.raw(),
                self.span(),
            )),
        }
    }

    /// Attempts to parse the wrapped value
    /// to a given prisma type.
    pub fn as_type(&self, scalar_type: ScalarType) -> Result<dml::ScalarValue, DatamodelError> {
        match scalar_type {
            ScalarType::Int => self.as_int().map(dml::ScalarValue::Int),
            ScalarType::Float => self.as_float().map(dml::ScalarValue::Float),
            ScalarType::Decimal => self.as_decimal().map(dml::ScalarValue::Decimal),
            ScalarType::Boolean => self.as_bool().map(dml::ScalarValue::Boolean),
            ScalarType::DateTime => self.as_date_time().map(dml::ScalarValue::DateTime),
            ScalarType::String => self.as_str().map(dml::ScalarValue::String),
        }
    }

    /// Parses the wrapped value as a given literal type.
    pub fn parse_literal<T: FromStrAndSpan>(&self) -> Result<T, DatamodelError> {
        T::from_str_and_span(&self.as_constant_literal()?, self.span())
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
    pub fn as_str(&self) -> Result<String, DatamodelError> {
        self.as_str_from_env().map(|tuple| tuple.1)
    }

    /// returns a (Some(a), b) if the string was deducted from an env var
    pub fn as_str_from_env(&self) -> Result<(Option<String>, String), DatamodelError> {
        match &self.value {
            ast::Expression::Function(name, _, _) if name == "env" => {
                let env_function = self.as_env_function()?;
                let var_name = Some(env_function.var_name().to_string());
                let value = env_function.evaluate().and_then(|x| x.as_str())?;
                Ok((var_name, value))
            }
            ast::Expression::StringValue(value, _) => Ok((None, value.to_string())),
            _ => Err(self.construct_type_mismatch_error("String")),
        }
    }

    pub fn as_env_function(&self) -> Result<EnvFunction, DatamodelError> {
        EnvFunction::from_ast(&self.value)
    }

    /// returns true if this argument is derived from an env() function
    pub fn is_from_env(&self) -> bool {
        self.value.is_env_expression()
    }

    /// Tries to convert the wrapped value to a Prisma Integer.
    pub fn as_int(&self) -> Result<i32, DatamodelError> {
        match &self.value {
            ast::Expression::NumericValue(value, _) => self.wrap_error_from_result(value.parse::<i32>(), "numeric"),
            ast::Expression::Any(value, _) => self.wrap_error_from_result(value.parse::<i32>(), "numeric"),
            _ => Err(self.construct_type_mismatch_error("numeric")),
        }
    }

    /// Tries to convert the wrapped value to a Prisma Float.
    pub fn as_float(&self) -> Result<f32, DatamodelError> {
        match &self.value {
            ast::Expression::NumericValue(value, _) => self.wrap_error_from_result(value.parse::<f32>(), "numeric"),
            ast::Expression::Any(value, _) => self.wrap_error_from_result(value.parse::<f32>(), "numeric"),
            _ => Err(self.construct_type_mismatch_error("numeric")),
        }
    }

    // TODO: Ask which decimal type to take.
    /// Tries to convert the wrapped value to a Prisma Decimal.
    pub fn as_decimal(&self) -> Result<f32, DatamodelError> {
        match &self.value {
            ast::Expression::NumericValue(value, _) => self.wrap_error_from_result(value.parse::<f32>(), "numeric"),
            ast::Expression::Any(value, _) => self.wrap_error_from_result(value.parse::<f32>(), "numeric"),
            _ => Err(self.construct_type_mismatch_error("numeric")),
        }
    }

    /// Tries to convert the wrapped value to a Prisma Boolean.
    pub fn as_bool(&self) -> Result<bool, DatamodelError> {
        match &self.value {
            ast::Expression::BooleanValue(value, _) => self.wrap_error_from_result(value.parse::<bool>(), "boolean"),
            ast::Expression::Any(value, _) => self.wrap_error_from_result(value.parse::<bool>(), "boolean"),
            // this case is just here because `as_bool_from_env` passes a StringValue
            ast::Expression::StringValue(value, _) => self.wrap_error_from_result(value.parse::<bool>(), "boolean"),
            _ => Err(self.construct_type_mismatch_error("boolean")),
        }
    }

    /// returns a (Some(a), _) if the value comes from an env var
    /// returns a (_, Some(b)) if the value could be parsed into a bool
    pub fn as_bool_from_env(&self) -> Result<(Option<String>, Option<bool>), DatamodelError> {
        match &self.value {
            ast::Expression::Function(name, _, _) if name == "env" => {
                let env_function = self.as_env_function()?;
                let var_name = if env_function.is_var_defined() {
                    Some(env_function.var_name().to_string())
                } else {
                    None
                };

                let value = env_function.evaluate().and_then(|x| x.as_bool()).ok();
                Ok((var_name, value))
            }
            ast::Expression::BooleanValue(value, _) => Ok((
                None,
                Some(self.wrap_error_from_result(value.parse::<bool>(), "boolean")?),
            )),
            _ => Err(self.construct_type_mismatch_error("String")),
        }
    }

    /// Tries to convert the wrapped value to a Prisma DateTime.
    pub fn as_date_time(&self) -> Result<DateTime<Utc>, DatamodelError> {
        match &self.value {
            ast::Expression::StringValue(value, _) => {
                self.wrap_error_from_result(value.parse::<DateTime<Utc>>(), "datetime")
            }
            ast::Expression::Any(value, _) => self.wrap_error_from_result(value.parse::<DateTime<Utc>>(), "datetime"),
            _ => Err(self.construct_type_mismatch_error("dateTime")),
        }
    }

    /// Unwraps the wrapped value as a constant literal..
    pub fn as_constant_literal(&self) -> Result<String, DatamodelError> {
        match &self.value {
            ast::Expression::ConstantValue(value, _) => Ok(value.to_string()),
            ast::Expression::Any(value, _) => Ok(value.to_string()),
            _ => Err(self.construct_type_mismatch_error("constant literal")),
        }
    }

    /// Unwraps the wrapped value as a constant literal..
    pub fn as_array(&self) -> Result<Vec<ValueValidator>, DatamodelError> {
        match &self.value {
            ast::Expression::Array(values, _) => {
                let mut validators: Vec<ValueValidator> = Vec::new();

                for value in values {
                    validators.push(ValueValidator::new(value));
                }

                Ok(validators)
            }
            _ => Ok(vec![ValueValidator {
                value: self.value.clone(),
            }]),
        }
    }

    pub fn as_default_value(&self, scalar_type: ScalarType) -> Result<DefaultValue, DatamodelError> {
        match &self.value {
            ast::Expression::Function(name, _, _) => {
                Ok(DefaultValue::Expression(ValueGenerator::new(name.to_string(), vec![])?))
            }
            _ => {
                let x = ValueValidator::new(&self.value).as_type(scalar_type)?;
                Ok(DefaultValue::Single(x))
            }
        }
    }
}

pub trait ValueListValidator {
    fn to_str_vec(&self) -> Result<Vec<String>, DatamodelError>;
    fn to_literal_vec(&self) -> Result<Vec<String>, DatamodelError>;
}

impl ValueListValidator for Vec<ValueValidator> {
    fn to_str_vec(&self) -> Result<Vec<String>, DatamodelError> {
        let mut res: Vec<String> = Vec::new();

        for val in self {
            res.push(val.as_str()?);
        }

        Ok(res)
    }

    fn to_literal_vec(&self) -> Result<Vec<String>, DatamodelError> {
        let mut res: Vec<String> = Vec::new();

        for val in self {
            res.push(val.as_constant_literal()?);
        }

        Ok(res)
    }
}
