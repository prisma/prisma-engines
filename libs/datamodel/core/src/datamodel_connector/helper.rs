use diagnostics::{DatamodelError, Span};
use std::error;

pub fn wrap_error_from_result<T, E: error::Error>(
    result: Result<T, E>,
    expected_type: &str,
    raw: &str,
    span: Span,
) -> Result<T, DatamodelError> {
    match result {
        Ok(val) => Ok(val),
        Err(err) => Err(DatamodelError::new_value_parser_error(
            expected_type,
            &err.to_string(),
            raw,
            span,
        )),
    }
}

pub fn parse_one_u32(args: Vec<String>, type_name: &str, span: Span) -> Result<u32, DatamodelError> {
    let number_of_args = args.len();

    match parse_u32_arguments(args, span)?.as_slice() {
        [x] => Ok(*x),
        _ => Err(DatamodelError::new_native_type_argument_count_mismatch_error(
            type_name,
            1,
            number_of_args,
            span,
        )),
    }
}

pub fn parse_one_opt_u32(args: Vec<String>, type_name: &str, span: Span) -> Result<Option<u32>, DatamodelError> {
    let number_of_args = args.len();

    match parse_u32_arguments(args, span)?.as_slice() {
        [x] => Ok(Some(*x)),
        [] => Ok(None),
        _ => Err(DatamodelError::new_native_type_argument_count_mismatch_error(
            type_name,
            1,
            number_of_args,
            span,
        )),
    }
}

pub fn parse_two_opt_u32(args: Vec<String>, type_name: &str, span: Span) -> Result<Option<(u32, u32)>, DatamodelError> {
    let number_of_args = args.len();

    match parse_u32_arguments(args, span)?.as_slice() {
        [x, y] => Ok(Some((*x, *y))),
        [] => Ok(None),
        _ => Err(DatamodelError::new_native_type_argument_count_mismatch_error(
            type_name,
            2,
            number_of_args,
            span,
        )),
    }
}

fn parse_u32_arguments(args: Vec<String>, span: Span) -> Result<Vec<u32>, DatamodelError> {
    let res = args
        .iter()
        .map(|arg| wrap_error_from_result(arg.parse::<i64>(), "numeric", arg, span))
        .collect::<Vec<_>>();

    if let Some(error) = res.iter().find(|arg| arg.is_err()) {
        Err(error.clone().err().unwrap())
    } else {
        Ok(res.iter().map(|arg| *arg.as_ref().unwrap() as u32).collect::<Vec<_>>())
    }
}

pub fn arg_vec_from_opt(input: Option<u32>) -> Vec<String> {
    input.into_iter().map(|x| x.to_string()).collect()
}

pub fn args_vec_from_opt(input: Option<(u32, u32)>) -> Vec<String> {
    match input {
        Some((x, y)) => vec![x.to_string(), y.to_string()],
        None => vec![],
    }
}
