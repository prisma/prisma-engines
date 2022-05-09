//! See [ValueValidator](./struct.ValueValidator.html).

use crate::{
    ast::{self, Span},
    relations::ReferentialAction,
    types::SortOrder,
};
use diagnostics::DatamodelError;
use std::{error, fmt};

/// Wraps a value and provides convenience methods for
/// validating it.
pub struct ValueValidator<'a> {
    /// The underlying AST expression.
    pub value: &'a ast::Expression,
}

impl<'a> fmt::Debug for ValueValidator<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ValueValidator").field("value", self.value).finish()
    }
}

pub(crate) enum OperatorClass<'a> {
    Constant(crate::OperatorClass),
    Raw(&'a str),
}

impl<'a> From<crate::OperatorClass> for OperatorClass<'a> {
    fn from(inner: crate::OperatorClass) -> Self {
        Self::Constant(inner)
    }
}

#[derive(Default)]
pub(super) struct IndexFieldAttributes<'a> {
    pub(super) field_name: &'a str,
    pub(super) sort_order: Option<SortOrder>,
    pub(super) length: Option<u32>,
    pub(super) operator_class: Option<OperatorClass<'a>>,
}

struct FieldArguments<'a> {
    sort_order: Option<SortOrder>,
    length: Option<u32>,
    operator_class: Option<OperatorClass<'a>>,
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

    /// Returns true if this argument is derived from an env() function
    pub fn is_from_env(&self) -> bool {
        self.value.is_env_expression()
    }

    /// Tries to convert the wrapped value to a Prisma Integer.
    pub fn as_int(&self) -> Result<i64, DatamodelError> {
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

    /// Unwraps the wrapped value as a boolean.
    pub fn as_bool(&self) -> Result<bool, DatamodelError> {
        match self.as_constant_literal() {
            Ok("true") => Ok(true),
            Ok("false") => Ok(false),
            _ => Err(self.construct_type_mismatch_error("boolean")),
        }
    }

    #[allow(clippy::type_complexity)]
    /// Unwraps the value as an array of constants.
    pub(crate) fn as_field_array_with_args(&self) -> Result<Vec<IndexFieldAttributes<'a>>, DatamodelError> {
        if let ast::Expression::Array(values, _) = &self.value {
            values.iter().map(|val| ValueValidator::new(val).as_func()).collect()
        } else {
            // Single values are accepted as array literals, for example in `@relation(fields: userId)`.
            Ok(vec![self.as_func()?])
        }
    }

    fn as_func(&self) -> Result<IndexFieldAttributes<'a>, DatamodelError> {
        match &self.value {
            ast::Expression::ConstantValue(field_name, _) => Ok(IndexFieldAttributes {
                field_name,
                ..Default::default()
            }),
            ast::Expression::Function(field_name, args, _) => {
                let args = ValueValidator::field_args(&args.arguments)?;

                let attrs = IndexFieldAttributes {
                    field_name,
                    sort_order: args.sort_order,
                    length: args.length,
                    operator_class: args.operator_class,
                };

                Ok(attrs)
            }

            _ => Err(self.construct_type_mismatch_error("constant literal")),
        }
    }

    fn field_args(args: &'a [ast::Argument]) -> Result<FieldArguments<'a>, DatamodelError> {
        let sort_order = args
            .iter()
            .find(|arg| arg.name.as_ref().map(|n| n.name.as_str()) == Some("sort"))
            .map(|arg| match arg.value.as_constant_value() {
                Some(("Asc", _)) => Ok(Some(SortOrder::Asc)),
                Some(("Desc", _)) => Ok(Some(SortOrder::Desc)),
                None => Ok(None),
                _ => Err(DatamodelError::new_parser_error("Asc, Desc".to_owned(), arg.span)),
            })
            .transpose()?
            .flatten();

        let length = args
            .iter()
            .find(|arg| arg.name.as_ref().map(|n| n.name.as_str()) == Some("length"))
            .map(|arg| match &arg.value {
                ast::Expression::NumericValue(s, _) => s
                    .parse::<u32>()
                    .map_err(|_| DatamodelError::new_parser_error("valid integer".to_owned(), arg.span)),
                _ => Err(DatamodelError::new_parser_error("valid integer".to_owned(), arg.span)),
            })
            .transpose()?;

        let operator_class = args
            .iter()
            .find(|arg| arg.name.as_ref().map(|n| n.name.as_str()) == Some("ops"))
            .map(|arg| match &arg.value {
                ast::Expression::ConstantValue(s, span) => match s.as_str() {
                    // gist
                    "InetOps" => Ok(OperatorClass::from(crate::OperatorClass::InetOps)),

                    // gin
                    "JsonbOps" => Ok(OperatorClass::from(crate::OperatorClass::JsonbOps)),
                    "JsonbPathOps" => Ok(OperatorClass::from(crate::OperatorClass::JsonbPathOps)),
                    "ArrayOps" => Ok(OperatorClass::from(crate::OperatorClass::ArrayOps)),

                    // sp-gist
                    "TextOps" => Ok(OperatorClass::from(crate::OperatorClass::TextOps)),

                    // brin
                    "BitMinMaxOps" => Ok(OperatorClass::from(crate::OperatorClass::BitMinMaxOps)),
                    "VarBitMinMaxOps" => Ok(OperatorClass::from(crate::OperatorClass::VarBitMinMaxOps)),
                    "BpcharBloomOps" => Ok(OperatorClass::from(crate::OperatorClass::BpcharBloomOps)),
                    "BpcharMinMaxOps" => Ok(OperatorClass::from(crate::OperatorClass::BpcharMinMaxOps)),
                    "ByteaBloomOps" => Ok(OperatorClass::from(crate::OperatorClass::ByteaBloomOps)),
                    "ByteaMinMaxOps" => Ok(OperatorClass::from(crate::OperatorClass::ByteaMinMaxOps)),
                    "DateBloomOps" => Ok(OperatorClass::from(crate::OperatorClass::DateBloomOps)),
                    "DateMinMaxOps" => Ok(OperatorClass::from(crate::OperatorClass::DateMinMaxOps)),
                    "DateMinMaxMultiOps" => Ok(OperatorClass::from(crate::OperatorClass::DateMinMaxMultiOps)),
                    "Float4BloomOps" => Ok(OperatorClass::from(crate::OperatorClass::Float4BloomOps)),
                    "Float4MinMaxOps" => Ok(OperatorClass::from(crate::OperatorClass::Float4MinMaxOps)),
                    "Float4MinMaxMultiOps" => Ok(OperatorClass::from(crate::OperatorClass::Float4MinMaxMultiOps)),
                    "Float8BloomOps" => Ok(OperatorClass::from(crate::OperatorClass::Float8BloomOps)),
                    "Float8MinMaxOps" => Ok(OperatorClass::from(crate::OperatorClass::Float8MinMaxOps)),
                    "Float8MinMaxMultiOps" => Ok(OperatorClass::from(crate::OperatorClass::Float8MinMaxMultiOps)),
                    "InetInclusionOps" => Ok(OperatorClass::from(crate::OperatorClass::InetInclusionOps)),
                    "InetBloomOps" => Ok(OperatorClass::from(crate::OperatorClass::InetBloomOps)),
                    "InetMinMaxOps" => Ok(OperatorClass::from(crate::OperatorClass::InetMinMaxOps)),
                    "InetMinMaxMultiOps" => Ok(OperatorClass::from(crate::OperatorClass::InetMinMaxMultiOps)),
                    "Int2BloomOps" => Ok(OperatorClass::from(crate::OperatorClass::Int2BloomOps)),
                    "Int2MinMaxOps" => Ok(OperatorClass::from(crate::OperatorClass::Int2MinMaxOps)),
                    "Int2MinMaxMultiOps" => Ok(OperatorClass::from(crate::OperatorClass::Int2MinMaxMultiOps)),
                    "Int4BloomOps" => Ok(OperatorClass::from(crate::OperatorClass::Int4BloomOps)),
                    "Int4MinMaxOps" => Ok(OperatorClass::from(crate::OperatorClass::Int4MinMaxOps)),
                    "Int4MinMaxMultiOps" => Ok(OperatorClass::from(crate::OperatorClass::Int4MinMaxMultiOps)),
                    "Int8BloomOps" => Ok(OperatorClass::from(crate::OperatorClass::Int8BloomOps)),
                    "Int8MinMaxOps" => Ok(OperatorClass::from(crate::OperatorClass::Int8MinMaxOps)),
                    "Int8MinMaxMultiOps" => Ok(OperatorClass::from(crate::OperatorClass::Int8MinMaxMultiOps)),
                    "NumericBloomOps" => Ok(OperatorClass::from(crate::OperatorClass::NumericBloomOps)),
                    "NumericMinMaxOps" => Ok(OperatorClass::from(crate::OperatorClass::NumericMinMaxOps)),
                    "NumericMinMaxMultiOps" => Ok(OperatorClass::from(crate::OperatorClass::NumericMinMaxMultiOps)),
                    "OidBloomOps" => Ok(OperatorClass::from(crate::OperatorClass::OidBloomOps)),
                    "OidMinMaxOps" => Ok(OperatorClass::from(crate::OperatorClass::OidMinMaxOps)),
                    "OidMinMaxMultiOps" => Ok(OperatorClass::from(crate::OperatorClass::OidMinMaxMultiOps)),
                    "TextBloomOps" => Ok(OperatorClass::from(crate::OperatorClass::TextBloomOps)),
                    "TextMinMaxOps" => Ok(OperatorClass::from(crate::OperatorClass::TextMinMaxOps)),
                    "TimestampBloomOps" => Ok(OperatorClass::from(crate::OperatorClass::TimestampBloomOps)),
                    "TimestampMinMaxOps" => Ok(OperatorClass::from(crate::OperatorClass::TimestampMinMaxOps)),
                    "TimestampMinMaxMultiOps" => Ok(OperatorClass::from(crate::OperatorClass::TimestampMinMaxMultiOps)),
                    "TimestampTzBloomOps" => Ok(OperatorClass::from(crate::OperatorClass::TimestampTzBloomOps)),
                    "TimestampTzMinMaxOps" => Ok(OperatorClass::from(crate::OperatorClass::TimestampTzMinMaxOps)),
                    "TimestampTzMinMaxMultiOps" => {
                        Ok(OperatorClass::from(crate::OperatorClass::TimestampTzMinMaxMultiOps))
                    }
                    "TimeBloomOps" => Ok(OperatorClass::from(crate::OperatorClass::TimeBloomOps)),
                    "TimeMinMaxOps" => Ok(OperatorClass::from(crate::OperatorClass::TimeMinMaxOps)),
                    "TimeMinMaxMultiOps" => Ok(OperatorClass::from(crate::OperatorClass::TimeMinMaxMultiOps)),
                    "TimeTzBloomOps" => Ok(OperatorClass::from(crate::OperatorClass::TimeTzBloomOps)),
                    "TimeTzMinMaxOps" => Ok(OperatorClass::from(crate::OperatorClass::TimeTzMinMaxOps)),
                    "TimeTzMinMaxMultiOps" => Ok(OperatorClass::from(crate::OperatorClass::TimeTzMinMaxMultiOps)),
                    "UuidBloomOps" => Ok(OperatorClass::from(crate::OperatorClass::UuidBloomOps)),
                    "UuidMinMaxOps" => Ok(OperatorClass::from(crate::OperatorClass::UuidMinMaxOps)),
                    "UuidMinMaxMultiOps" => Ok(OperatorClass::from(crate::OperatorClass::UuidMinMaxMultiOps)),

                    s => Err(DatamodelError::new_parser_error(
                        format!("Invalid operator class: {s}"),
                        *span,
                    )),
                },
                ast::Expression::Function(fun, args, span) => match fun.as_str() {
                    "raw" => match args.arguments.as_slice() {
                        [arg] => match &arg.value {
                            ast::Expression::StringValue(s, _) => Ok(OperatorClass::Raw(s.as_str())),
                            _ => Err(DatamodelError::new_parser_error(
                                "Invalid parameter type: expected string".into(),
                                *span,
                            )),
                        },
                        args => Err(DatamodelError::new_parser_error(
                            format!("Wrong number of arguments. Expected: 1, got: {}", args.len()),
                            *span,
                        )),
                    },
                    _ => panic!(),
                },
                _ => Err(DatamodelError::new_parser_error("operator class".to_owned(), arg.span)),
            })
            .transpose()?;

        Ok(FieldArguments {
            sort_order,
            length,
            operator_class,
        })
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

                Err(DatamodelError::new_attribute_validation_error(
                    &message,
                    "@relation",
                    self.span(),
                ))
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
