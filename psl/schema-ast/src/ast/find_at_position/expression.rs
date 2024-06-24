use crate::ast::{self};

use super::WithSpan;

#[derive(Debug)]
pub enum ExpressionPosition<'ast> {
    Expression,
    Value(&'ast str),
    Function(&'ast str),
    FunctionArgument(&'ast str, &'ast str),
}

impl<'ast> ExpressionPosition<'ast> {
    pub(crate) fn new(expr: &'ast ast::Expression, position: usize) -> Self {
        match expr {
            ast::Expression::NumericValue(val, span) if span.contains(position) => Self::Value(val),
            ast::Expression::StringValue(val, span) if span.contains(position) => Self::Value(val),
            ast::Expression::ConstantValue(val, span) if span.contains(position) => Self::Value(val),
            ast::Expression::Function(name, args, span) if span.contains(position) => {
                narrow_function_position(args, position, name)
            }
            ast::Expression::Array(exprs, span) if span.contains(position) => {
                for expr in exprs.iter() {
                    match ExpressionPosition::new(expr, position) {
                        ExpressionPosition::Expression => (),
                        e => return e,
                    }
                }

                Self::Expression
            }
            _ => Self::Expression,
        }
    }
}

fn narrow_function_position<'ast>(
    args: &'ast ast::ArgumentsList,
    position: usize,
    name: &'ast str,
) -> ExpressionPosition<'ast> {
    let mut spans: Vec<(Option<&str>, ast::Span)> = args
        .arguments
        .iter()
        .map(|arg| (arg.name.as_ref().map(|n| n.name.as_str()), arg.span()))
        .chain(
            args.empty_arguments
                .iter()
                .map(|arg| (Some(arg.name.name.as_str()), arg.name.span())),
        )
        .collect();

    spans.sort_by_key(|(_, span)| span.start);

    let mut arg_name = None;
    for (name, _) in spans.iter().take_while(|(_, span)| span.start < position) {
        arg_name = Some(*name);
    }

    // If the cursor is after a trailing comma, we're not in an argument.
    if let Some(span) = args.trailing_comma {
        if position > span.start {
            arg_name = None;
        }
    }

    if let Some(arg_name) = arg_name.flatten() {
        ExpressionPosition::FunctionArgument(name, arg_name)
    } else {
        ExpressionPosition::Function(name)
    }
}
