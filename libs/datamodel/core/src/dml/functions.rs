use crate::ast;
use crate::common::value::ValueValidator;
use crate::error::DatamodelError;

//pub trait Function {
//    fn name(&self) -> &str;
//
//    fn apply(&self, args: &[ast::Expression], span: ast::Span) -> Result<MaybeExpression, DatamodelError>;
//}

pub struct EnvFunction {
    var_name: String,
    span: ast::Span,
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
        let var_name = ValueValidator::new(var_wrapped).as_str()?;
        Ok(Self {
            var_name,
            span: expr.span(),
        })
    }

    pub fn var_name(&self) -> &str {
        &self.var_name
    }

    pub fn is_var_defined(&self) -> bool {
        std::env::var(&self.var_name).is_ok()
    }

    pub fn evaluate(&self) -> Result<ValueValidator, DatamodelError> {
        if let Ok(var) = std::env::var(&self.var_name) {
            let value_validator = ValueValidator::new(&ast::Expression::StringValue(var, self.span));
            Ok(value_validator)
        } else {
            Err(DatamodelError::new_environment_functional_evaluation_error(
                &self.var_name,
                self.span,
            ))
        }
    }
}
