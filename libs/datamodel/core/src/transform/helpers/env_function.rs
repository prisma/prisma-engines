use crate::{ast, diagnostics::DatamodelError, parser_database::ValueValidator};

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

        Ok(Self {
            var_name: ValueValidator::new(&args[0].value).as_str()?.to_owned(),
        })
    }

    pub fn var_name(&self) -> &str {
        &self.var_name
    }
}
