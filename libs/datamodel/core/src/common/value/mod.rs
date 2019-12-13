use crate::ast;
use crate::dml;
use crate::error::DatamodelError;

use super::functions::FunctionalEvaluator;
use super::interpolation::StringInterpolator;
use super::FromStrAndSpan;
use super::{ScalarType, ScalarValue};
use chrono::{DateTime, Utc};
use std::error;

#[derive(Debug, Clone)]
pub enum MaybeExpression {
    // The Option is Some if the value came from an env var. The String is then the name of the env var.
    Value(Option<String>, ast::Expression),
    Expression(ScalarValue, ast::Span),
}

/// Wraps a value and provides convenience methods for
/// parsing it.
#[derive(Debug)]
pub struct ValueValidator {
    value: MaybeExpression,
}

impl ValueValidator {
    /// Creates a new instance by wrapping a value.
    ///
    /// If the value is a function expression, it is evaluated
    /// recursively.
    pub fn new(value: &ast::Expression) -> Result<ValueValidator, DatamodelError> {
        match value {
            ast::Expression::StringValue(string, span) => Ok(ValueValidator {
                value: MaybeExpression::Value(None, StringInterpolator::interpolate(string, *span)?),
            }),
            _ => Ok(ValueValidator {
                value: FunctionalEvaluator::new(value).evaluate()?,
            }),
        }
    }

    /// Creates a new type mismatch error for the
    /// value wrapped by this instance.
    fn construct_error(&self, expected_type: &str) -> DatamodelError {
        let description = match &self.value {
            MaybeExpression::Value(_, val) => String::from(ast::describe_value_type(&val)),
            MaybeExpression::Expression(val, _) => val.get_type().to_string(),
        };

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
        match &self.value {
            MaybeExpression::Value(_, _) => match scalar_type {
                ScalarType::Int => self.as_int().map(dml::ScalarValue::Int),
                ScalarType::Float => self.as_float().map(dml::ScalarValue::Float),
                ScalarType::Decimal => self.as_decimal().map(dml::ScalarValue::Decimal),
                ScalarType::Boolean => self.as_bool().map(dml::ScalarValue::Boolean),
                ScalarType::DateTime => self.as_date_time().map(dml::ScalarValue::DateTime),
                ScalarType::String => self.as_str().map(dml::ScalarValue::String),
            },
            MaybeExpression::Expression(expr, _) => {
                if expr.get_type() == scalar_type {
                    Ok(expr.clone())
                } else {
                    Err(self.construct_error(&scalar_type.to_string()))
                }
            }
        }
    }

    /// Parses the wrapped value as a given literal type.
    pub fn parse_literal<T: FromStrAndSpan>(&self) -> Result<T, DatamodelError> {
        T::from_str_and_span(&self.as_constant_literal()?, self.span())
    }

    /// Accesses the raw string representation
    /// of the wrapped value.
    pub fn raw(&self) -> String {
        match &self.value {
            MaybeExpression::Value(_, val) => val.to_string(),
            MaybeExpression::Expression(val, _) => val.to_string(),
        }
    }

    /// Accesses the span of the wrapped value.
    pub fn span(&self) -> ast::Span {
        match &self.value {
            MaybeExpression::Value(_, val) => match val {
                ast::Expression::StringValue(_, s) => *s,
                ast::Expression::NumericValue(_, s) => *s,
                ast::Expression::BooleanValue(_, s) => *s,
                ast::Expression::ConstantValue(_, s) => *s,
                ast::Expression::Function(_, _, s) => *s,
                ast::Expression::Array(_, s) => *s,
                ast::Expression::Any(_, s) => *s,
            },
            MaybeExpression::Expression(_, s) => *s,
        }
    }

    /// Tries to convert the wrapped value to a Prisma String.
    pub fn as_str(&self) -> Result<String, DatamodelError> {
        self.as_str_from_env().map(|tuple| tuple.1)
    }

    /// returns a (Some(a), b) if the string was deducted from an env var
    pub fn as_str_from_env(&self) -> Result<(Option<String>, String), DatamodelError> {
        match &self.value {
            MaybeExpression::Value(env_var, ast::Expression::StringValue(value, _)) => {
                Ok((env_var.clone(), value.to_string()))
            }
            MaybeExpression::Value(env_var, ast::Expression::Any(value, _)) => Ok((env_var.clone(), value.to_string())),
            _ => Err(self.construct_error("String")),
        }
    }

    /// returns true if this argument is derived from an env() function
    pub fn is_from_env(&self) -> bool {
        match &self.value {
            MaybeExpression::Value(Some(_), _) => true,
            _ => false,
        }
    }

    /// Tries to convert the wrapped value to a Prisma Integer.
    pub fn as_int(&self) -> Result<i32, DatamodelError> {
        match &self.value {
            MaybeExpression::Value(_, ast::Expression::NumericValue(value, _)) => {
                self.wrap_error_from_result(value.parse::<i32>(), "numeric")
            }
            MaybeExpression::Value(_, ast::Expression::Any(value, _)) => {
                self.wrap_error_from_result(value.parse::<i32>(), "numeric")
            }
            _ => Err(self.construct_error("numeric")),
        }
    }

    /// Tries to convert the wrapped value to a Prisma Float.
    pub fn as_float(&self) -> Result<f32, DatamodelError> {
        match &self.value {
            MaybeExpression::Value(_, ast::Expression::NumericValue(value, _)) => {
                self.wrap_error_from_result(value.parse::<f32>(), "numeric")
            }
            MaybeExpression::Value(_, ast::Expression::Any(value, _)) => {
                self.wrap_error_from_result(value.parse::<f32>(), "numeric")
            }
            _ => Err(self.construct_error("numeric")),
        }
    }

    // TODO: Ask which decimal type to take.
    /// Tries to convert the wrapped value to a Prisma Decimal.
    pub fn as_decimal(&self) -> Result<f32, DatamodelError> {
        match &self.value {
            MaybeExpression::Value(_, ast::Expression::NumericValue(value, _)) => {
                self.wrap_error_from_result(value.parse::<f32>(), "numeric")
            }
            MaybeExpression::Value(_, ast::Expression::Any(value, _)) => {
                self.wrap_error_from_result(value.parse::<f32>(), "numeric")
            }
            _ => Err(self.construct_error("numeric")),
        }
    }

    /// Tries to convert the wrapped value to a Prisma Boolean.
    pub fn as_bool(&self) -> Result<bool, DatamodelError> {
        match &self.value {
            MaybeExpression::Value(_, ast::Expression::BooleanValue(value, _)) => {
                self.wrap_error_from_result(value.parse::<bool>(), "boolean")
            }
            MaybeExpression::Value(_, ast::Expression::Any(value, _)) => {
                self.wrap_error_from_result(value.parse::<bool>(), "boolean")
            }
            _ => Err(self.construct_error("boolean")),
        }
    }

    /// returns a (Some(a), _) if the value comes from an env var
    /// returns a (_, Some(b)) if the value could be parsed into a bool
    pub fn as_bool_from_env(&self) -> Result<(Option<String>, Option<bool>), DatamodelError> {
        match &self.value {
            MaybeExpression::Value(env_var, ast::Expression::BooleanValue(value, _)) => {
                let parsed_result = self.wrap_error_from_result(value.parse::<bool>(), "boolean");
                let the_bool = parsed_result.ok();
                Ok((env_var.clone(), the_bool))
            }
            MaybeExpression::Value(env_var, ast::Expression::Any(value, _)) => {
                let parsed_result = self.wrap_error_from_result(value.parse::<bool>(), "boolean");
                let the_bool = parsed_result.ok();
                Ok((env_var.clone(), the_bool))
            }
            _ => Err(self.construct_error("boolean")),
        }
    }

    // TODO: Ask which datetime type to use.
    /// Tries to convert the wrapped value to a Prisma DateTime.
    pub fn as_date_time(&self) -> Result<DateTime<Utc>, DatamodelError> {
        match &self.value {
            MaybeExpression::Value(_, ast::Expression::StringValue(value, _)) => {
                self.wrap_error_from_result(value.parse::<DateTime<Utc>>(), "datetime")
            }
            MaybeExpression::Value(_, ast::Expression::Any(value, _)) => {
                self.wrap_error_from_result(value.parse::<DateTime<Utc>>(), "datetime")
            }
            _ => Err(self.construct_error("dateTime")),
        }
    }

    /// Unwraps the wrapped value as a constant literal..
    pub fn as_constant_literal(&self) -> Result<String, DatamodelError> {
        match &self.value {
            MaybeExpression::Value(_, ast::Expression::ConstantValue(value, _)) => Ok(value.to_string()),
            MaybeExpression::Value(_, ast::Expression::Any(value, _)) => Ok(value.to_string()),
            _ => Err(self.construct_error("constant literal")),
        }
    }

    /// Unwraps the wrapped value as a constant literal..
    pub fn as_array(&self) -> Result<Vec<ValueValidator>, DatamodelError> {
        match &self.value {
            MaybeExpression::Value(_, ast::Expression::Array(values, _)) => {
                let mut validators: Vec<ValueValidator> = Vec::new();

                for value in values {
                    validators.push(ValueValidator::new(value)?);
                }

                Ok(validators)
            }
            _ => Ok(vec![ValueValidator {
                value: self.value.clone(),
            }]),
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

impl Into<ast::Expression> for dml::ScalarValue {
    fn into(self) -> ast::Expression {
        (&self).into()
    }
}

impl Into<ast::Expression> for &dml::ScalarValue {
    fn into(self) -> ast::Expression {
        match self {
            dml::ScalarValue::Boolean(true) => ast::Expression::BooleanValue(String::from("true"), ast::Span::empty()),
            dml::ScalarValue::Boolean(false) => {
                ast::Expression::BooleanValue(String::from("false"), ast::Span::empty())
            }
            dml::ScalarValue::String(value) => ast::Expression::StringValue(value.clone(), ast::Span::empty()),
            dml::ScalarValue::ConstantLiteral(value) => {
                ast::Expression::ConstantValue(value.clone(), ast::Span::empty())
            }
            dml::ScalarValue::DateTime(value) => ast::Expression::ConstantValue(value.to_rfc3339(), ast::Span::empty()),
            dml::ScalarValue::Decimal(value) => ast::Expression::NumericValue(value.to_string(), ast::Span::empty()),
            dml::ScalarValue::Float(value) => ast::Expression::NumericValue(value.to_string(), ast::Span::empty()),
            dml::ScalarValue::Int(value) => ast::Expression::NumericValue(value.to_string(), ast::Span::empty()),
            dml::ScalarValue::Expression(name, _, args) => ast::Expression::Function(
                name.clone(),
                args.iter().map(|a| a.into()).collect(),
                ast::Span::empty(),
            ),
        }
    }
}
