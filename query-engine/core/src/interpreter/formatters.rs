use super::Expression;
use crate::{Query, WriteQuery};

pub fn format_expression(expr: &Expression, indent: usize) -> String {
    match expr {
        Expression::Sequence { seq } => seq
            .into_iter()
            .map(|expr| add_indent(indent, format_expression(expr, indent + 2)))
            .collect::<Vec<String>>()
            .join("\n"),

        Expression::Query { query } => match query {
            Query::Read(rq) => add_indent(indent, format!("{}", rq)),
            Query::Write(WriteQuery::Root(wq)) => add_indent(indent, format!("{}", wq)),
            _ => unreachable!(),
        },

        Expression::Func { func: _ } => add_indent(indent, "(Fn env)"),
        Expression::Let { bindings, expressions } => {
            let binding_strs = bindings
                .into_iter()
                .map(|binding| {
                    add_indent(
                        indent + 2,
                        format!("(\"{}\" {})", binding.name, format_expression(&binding.expr, 0)),
                    )
                })
                .collect::<Vec<String>>()
                .join("\n");

            let exp_strs = expressions
                .into_iter()
                .map(|exp| format_expression(exp, indent))
                .collect::<Vec<String>>()
                .join("\n");

            format!("(Let [\n{}\n{}]\n{}\n)", binding_strs, indent_string(indent), exp_strs)
        }
        Expression::Get { binding_name } => add_indent(indent, format!("(Get env '{}')", binding_name)),
        Expression::GetFirstNonEmpty { binding_names } => {
            add_indent(indent, format!("(GetFirstNoneEmpty env '{:?}')", binding_names))
        }
        Expression::If {
            func: _,
            then: _,
            else_: _,
        } => add_indent(indent, "if (Fn env)"),
    }
}

fn add_indent<T: AsRef<str>>(indent: usize, s: T) -> String {
    format!("{}{}", indent_string(indent), s.as_ref())
}

fn indent_string(indent: usize) -> String {
    " ".repeat(indent)
}