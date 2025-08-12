use alloc::{
    borrow::ToOwned,
    string::{String, ToString},
};

use crate::parser_database::{ast, coerce};
use diagnostics::{DatamodelError, DatamodelWarning, Diagnostics};
use schema_ast::ast::WithSpan;
use serde::Serialize;

/// Either an env var or a string literal.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StringFromEnvVar {
    /// Contains the name of env var if the value was read from one.
    pub from_env_var: Option<String>,
    /// Contains the string literal, when it was directly in the parsed schema.
    pub value: Option<String>,
}

impl StringFromEnvVar {
    pub(crate) fn coerce(expr: &ast::Expression, diagnostics: &mut Diagnostics) -> Option<Self> {
        match expr {
            ast::Expression::Function(name, _, _) if name == "env" => EnvFunction::from_ast(expr, diagnostics)
                .map(|env_function| StringFromEnvVar::new_from_env_var(env_function.var_name().to_owned())),
            ast::Expression::StringValue(value, _) => Some(StringFromEnvVar::new_literal(value.clone())),
            _ => {
                diagnostics.push_error(DatamodelError::new_type_mismatch_error(
                    "String",
                    expr.describe_value_type(),
                    &expr.to_string(),
                    expr.span(),
                ));
                None
            }
        }
    }

    pub fn new_from_env_var(env_var_name: String) -> StringFromEnvVar {
        StringFromEnvVar {
            from_env_var: Some(env_var_name),
            value: None,
        }
    }

    pub fn new_literal(value: String) -> StringFromEnvVar {
        StringFromEnvVar {
            from_env_var: None,
            value: Some(value),
        }
    }

    /// Returns the name of the env var, if env var.
    pub fn as_env_var(&self) -> Option<&str> {
        self.from_env_var.as_deref()
    }

    /// Returns the contents of the string literal, if applicable.
    pub fn as_literal(&self) -> Option<&str> {
        self.value.as_deref()
    }
}

pub(crate) struct EnvFunction {
    var_name: String,
}

impl EnvFunction {
    pub(crate) fn from_ast(expr: &ast::Expression, diagnostics: &mut Diagnostics) -> Option<EnvFunction> {
        let args = if let ast::Expression::Function(name, args, _) = &expr {
            if name == "env" {
                args.arguments
                    .iter()
                    .filter(|arg| !arg.is_unnamed())
                    .for_each(|arg| diagnostics.push_warning(DatamodelWarning::new_named_env_val(arg.span())));

                if args.arguments.is_empty() && !args.empty_arguments.is_empty() {
                    diagnostics.push_error(DatamodelError::new_named_env_val(expr.span()));
                    return None;
                }

                args
            } else {
                diagnostics.push_error(DatamodelError::new_functional_evaluation_error(
                    "Expected this to be an env function.",
                    expr.span(),
                ));
                return None;
            }
        } else {
            diagnostics.push_error(DatamodelError::new_functional_evaluation_error(
                "This is not a function expression but expected it to be one.",
                expr.span(),
            ));
            return None;
        };

        if args.arguments.len() + args.empty_arguments.len() != 1 {
            diagnostics.push_error(DatamodelError::new_functional_evaluation_error(
                "Exactly one string parameter must be passed to the env function.",
                expr.span(),
            ));
            return None;
        }

        if args.trailing_comma.is_some() {
            diagnostics.push_error(DatamodelError::new_functional_evaluation_error(
                "Exactly one string parameter must be passed to the env function.",
                expr.span(),
            ));
            return None;
        }

        let var_wrapped = &args.arguments[0];
        let var_name = coerce::string(&var_wrapped.value, diagnostics)?.to_owned();

        Some(Self { var_name })
    }

    pub(crate) fn var_name(&self) -> &str {
        &self.var_name
    }
}
