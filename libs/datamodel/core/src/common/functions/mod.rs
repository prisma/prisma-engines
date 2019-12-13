mod builtin;
mod traits;

use crate::ast;
use crate::common::{
    value::{MaybeExpression, ValueValidator},
    ScalarType,
};
use crate::error::DatamodelError;

use traits::*;

// Client side funcs
const BUILTIN_ENV_FUNCTIONAL: builtin::EnvFunctional = builtin::EnvFunctional {};

// Server side funcs
const BUILTIN_NOW_FUNCTIONAL: builtin::ServerSideTrivialFunctional = builtin::ServerSideTrivialFunctional {
    name: "now",
    return_type: ScalarType::DateTime,
};
const BUILTIN_CUID_FUNCTIONAL: builtin::ServerSideTrivialFunctional = builtin::ServerSideTrivialFunctional {
    name: "cuid",
    return_type: ScalarType::String,
};
const BUILTIN_UUID_FUNCTIONAL: builtin::ServerSideTrivialFunctional = builtin::ServerSideTrivialFunctional {
    name: "uuid",
    return_type: ScalarType::String,
};
const BUILTIN_AUTOINCREMENT_FUNCTIONAL: builtin::ServerSideTrivialFunctional = builtin::ServerSideTrivialFunctional {
    name: "autoincrement",
    return_type: ScalarType::Int,
};

/// Array of all builtin functionals.
const BUILTIN_FUNCTIONALS: [&dyn Functional; 5] = [
    &BUILTIN_ENV_FUNCTIONAL,
    &BUILTIN_NOW_FUNCTIONAL,
    &BUILTIN_CUID_FUNCTIONAL,
    &BUILTIN_UUID_FUNCTIONAL,
    &BUILTIN_AUTOINCREMENT_FUNCTIONAL,
];

/// Evaluator for arbitrary expressions.
pub struct FunctionalEvaluator {
    value: ast::Expression,
}

impl FunctionalEvaluator {
    /// Wraps a value into a function evaluator.
    pub fn new(value: &ast::Expression) -> FunctionalEvaluator {
        FunctionalEvaluator { value: value.clone() }
    }

    /// Evaluates the value wrapped in this instance.
    ///
    /// If the value is of type Function, the corresponding function will
    /// be identified and and executed.
    ///
    /// Otherwise, if the value is a constant, the value is returned as-is.
    pub fn evaluate(&self) -> Result<MaybeExpression, DatamodelError> {
        match &self.value {
            ast::Expression::Function(name, params, span) => self.evaluate_functional(&name, &params, *span),
            _ => Ok(MaybeExpression::Value(None, self.value.clone())),
        }
    }

    fn evaluate_functional(
        &self,
        name: &str,
        args: &[ast::Expression],
        span: ast::Span,
    ) -> Result<MaybeExpression, DatamodelError> {
        for f in &BUILTIN_FUNCTIONALS {
            if f.name() == name {
                let mut resolved_args: Vec<ValueValidator> = Vec::new();

                for value in args {
                    resolved_args.push(ValueValidator::new(value)?)
                }

                return f.apply(&resolved_args, span);
            }
        }

        Err(DatamodelError::new_function_not_known_error(name, span))
    }
}
