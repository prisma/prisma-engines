use crate::parser_database::{ast, coerce};
use diagnostics::{DatamodelError, DatamodelWarning, Diagnostics};
use schema_ast::ast::WithSpan;
use serde::Serialize;

/// Either an env var or a string literal.
/// TODO: From Prisma 7 onwards, this struct will not be needed, as the value will always be a plain String.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StringFromEnvVar {
    /// Contains the name of env var if the value was read from one.
    pub from_env_var: Option<String>,
    /// Contains the string literal, when it was directly in the parsed schema.
    pub value: Option<String>,
    /// Contains the default value string if provided as the second argument to the `env()` function,
    /// used when the environment variable is not set.
    pub default: Option<String>,
}

impl StringFromEnvVar {
    pub(crate) fn coerce(expr: &ast::Expression, diagnostics: &mut Diagnostics) -> Option<Self> {
        match expr {
            ast::Expression::Function(name, _, _) if name == "env" => {
                let env_function = EnvFunction::from_ast(expr, diagnostics)?;
                let var_name = env_function.var_name().to_owned();
                let default_val = env_function.default().map(|s| s.to_owned());

                Some(StringFromEnvVar::new_from_env_var(var_name, default_val))
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

    pub fn new_from_env_var(env_var_name: String, default: Option<String>) -> StringFromEnvVar {
        StringFromEnvVar {
            from_env_var: Some(env_var_name),
            value: None,
            default,
        }
    }

    pub fn new_literal(value: String) -> StringFromEnvVar {
        StringFromEnvVar {
            from_env_var: None,
            value: Some(value),
            default: None,
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

    pub fn default(&self) -> Option<&str> {
        self.default.as_deref()
    }
}

pub(crate) struct EnvFunction {
    var_name: String,
    default: Option<String>,
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

        let (var_name_expr, default_value_expr) = match args.arguments.len() {
            1 => {
                let var_wrapped = &args.arguments[0];
                let var_name = coerce::string(&var_wrapped.value, diagnostics)?.to_owned();
                (var_name, None)
            }
            2 => {
                let var_wrapped = &args.arguments[0];
                let var_name = coerce::string(&var_wrapped.value, diagnostics)?.to_owned();

                let default_arg = &args.arguments[1];
                if default_arg.name.as_ref().map(|s| s.name.as_str()) != Some("default") {
                    diagnostics.push_error(DatamodelError::new_functional_evaluation_error(
                        "The second argument to env() must be named `default`.",
                        default_arg.span(),
                    ));
                    return None;
                }

                let default_value = coerce::string(&default_arg.value, diagnostics)?.to_owned();
                (var_name, Some(default_value))
            }
            _ => {
                diagnostics.push_error(DatamodelError::new_functional_evaluation_error(
                    "The `env` function takes one or two arguments. The first argument is the environment variable name, and the optional second argument is the default value, which must be a named argument `default`.",
                    expr.span(),
                ));
                return None;
            }
        };

        Some(Self {
            var_name: var_name_expr,
            default: default_value_expr,
        })
    }

    pub(crate) fn var_name(&self) -> &str {
        &self.var_name
    }

    pub(crate) fn default(&self) -> Option<&str> {
        self.default.as_deref()
    }
}
