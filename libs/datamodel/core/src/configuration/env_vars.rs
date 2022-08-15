use crate::parser_database::{ast, coerce};
use diagnostics::{DatamodelError, Diagnostics};
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
            ast::Expression::Function(name, _, _) if name == "env" => {
                let mut errs = Diagnostics::new();
                match EnvFunction::from_ast(expr, &mut errs) {
                    Some(env_function) => Some(StringFromEnvVar::new_from_env_var(env_function.var_name().to_owned())),
                    None => {
                        diagnostics.push_error(errs.errors()[0].clone());
                        None
                    }
                }
            }
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

struct EnvFunction {
    var_name: String,
}

impl EnvFunction {
    fn from_ast(expr: &ast::Expression, diagnostics: &mut Diagnostics) -> Option<EnvFunction> {
        let args = if let ast::Expression::Function(name, args, _) = &expr {
            if name == "env" {
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

        if args.arguments.len() != 1 {
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

    fn var_name(&self) -> &str {
        &self.var_name
    }
}
