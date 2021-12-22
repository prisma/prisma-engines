use crate::{ast, diagnostics::DatamodelError};

pub(crate) struct EnvFunction {
    var_name: String,
}

impl EnvFunction {
    pub fn from_ast(expr: &ast::Expression) -> Result<EnvFunction, DatamodelError> {
        let args = if let ast::Expression::Function(name, args, _) = &expr {
            if name == "env" {
                args
            } else {
                return Err(DatamodelError::new_functional_evaluation_error(
                    "Expected this to be an env function.",
                    expr.span(),
                ));
            }
        } else {
            return Err(DatamodelError::new_functional_evaluation_error(
                "This is not a function expression but expected it to be one.",
                expr.span(),
            ));
        };

        if args.len() != 1 {
            return Err(DatamodelError::new_functional_evaluation_error(
                "Exactly one string parameter must be passed to the env function.",
                expr.span(),
            ));
        }

        let var_wrapped = &args[0];
        if let Some((var_name, _)) = var_wrapped.as_string_value() {
            Ok(Self {
                var_name: var_name.to_owned(),
            })
        } else {
            Err(DatamodelError::new_validation_error(
                "The `env` function takes a single string argument.".to_owned(),
                expr.span(),
            ))
        }
    }

    pub fn var_name(&self) -> &str {
        &self.var_name
    }
}
