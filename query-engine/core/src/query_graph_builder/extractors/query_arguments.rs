use super::*;
use crate::{
    query_document::{ParsedArgument, ParsedInputMap},
    QueryGraphBuilderError, QueryGraphBuilderResult,
};
use connector::QueryArguments;
use prisma_models::{ModelRef, PrismaValue, ScalarFieldRef};
use std::convert::TryInto;

/// Expects the caller to know that it is structurally guaranteed that query arguments can be extracted,
/// e.g. that the query schema guarantees that required fields are present.
/// Errors occur if conversions fail unexpectedly.
pub fn extract_query_args(arguments: Vec<ParsedArgument>, model: &ModelRef) -> QueryGraphBuilderResult<QueryArguments> {
    arguments
        .into_iter()
        .fold(Ok(QueryArguments::default()), |result, arg| {
            if let Ok(res) = result {
                match arg.name.as_str() {
                    "skip" => Ok(QueryArguments {
                        skip: arg.value.try_into()?,
                        ..res
                    }),

                    "first" => Ok(QueryArguments {
                        first: arg.value.try_into()?,
                        ..res
                    }),

                    "last" => Ok(QueryArguments {
                        last: arg.value.try_into()?,
                        ..res
                    }),

                    "after" => Ok(QueryArguments {
                        after: extract_cursor(arg.value, model)?,
                        ..res
                    }),

                    "before" => Ok(QueryArguments {
                        before: extract_cursor(arg.value, model)?,
                        ..res
                    }),

                    "orderBy" => Ok(QueryArguments {
                        order_by: Some(arg.value.try_into()?),
                        ..res
                    }),

                    "where" => {
                        let val: Option<ParsedInputMap> = arg.value.try_into()?;
                        match val {
                            Some(m) => {
                                let filter = Some(extract_filter(m, model, true)?);
                                Ok(QueryArguments { filter, ..res })
                            }
                            None => Ok(res),
                        }
                    }

                    _ => Ok(res),
                }
            } else {
                result
            }
        })
}

fn extract_cursor(
    value: ParsedInputValue,
    model: &ModelRef,
) -> QueryGraphBuilderResult<Option<Vec<(ScalarFieldRef, PrismaValue)>>> {
    if let Err(_) = value.assert_non_null() {
        return Ok(None);
    }

    let map: ParsedInputMap = value.try_into()?;
    map.assert_size(1)?;

    let (field_name, value): (String, ParsedInputValue) = map.into_iter().nth(0).unwrap();

    // Always try to resolve regular fields first. If that fails, try to resolve compound fields.
    model
        .fields()
        .find_from_scalar(&field_name)
        .map_err(|err| err.into())
        .and_then(|field| {
            let value: PrismaValue = value.clone().try_into()?;
            Ok(Some(vec![(field, value)]))
        })
        .or_else(|_: QueryGraphBuilderError| {
            utils::resolve_compound_field(&field_name, &model)
                .ok_or(QueryGraphBuilderError::AssertionError(format!(
                    "Unable to resolve field {} to a field or a set of fields on model {}",
                    field_name, model.name
                )))
                .and_then(|fields| {
                    let mut compound_map: ParsedInputMap = value.try_into()?;
                    let mut result = vec![];

                    for field in fields {
                        // Unwrap is safe because validation gurantees that the value is present.
                        let value = compound_map.remove(&field.name).unwrap().try_into()?;
                        result.push((field, value));
                    }

                    Ok(Some(result))
                })
        })
}
