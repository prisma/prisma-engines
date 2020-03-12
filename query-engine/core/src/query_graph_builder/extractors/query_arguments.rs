use super::*;
use crate::{
    query_document::{ParsedArgument, ParsedInputMap},
    QueryGraphBuilderError, QueryGraphBuilderResult,
};
use connector::QueryArguments;
use prisma_models::{DataSourceFieldRef, Field, ModelRef, PrismaValue, RecordProjection};
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
                                let filter = Some(extract_filter(m, model)?);
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

fn extract_cursor(value: ParsedInputValue, model: &ModelRef) -> QueryGraphBuilderResult<Option<RecordProjection>> {
    if let Err(_) = value.assert_non_null() {
        return Ok(None);
    }

    let input_map: ParsedInputMap = value.try_into()?;
    let mut pairs = vec![];

    for (field_name, map_value) in input_map {
        let additional_pairs = match model.fields().find_from_all(&field_name) {
            Ok(field) => extract_cursor_field(field, map_value)?,
            Err(_) => match utils::resolve_compound_field(&field_name, &model) {
                Some(fields) => extract_compound_cursor_field(fields, map_value)?,
                None => Err(QueryGraphBuilderError::AssertionError(format!(
                    "Unable to resolve field {} to a field or a set of fields on model {}",
                    field_name, model.name
                )))?,
            },
        };

        pairs.extend(additional_pairs);
    }

    Ok(Some(RecordProjection::new(pairs)))
}

fn extract_cursor_field(
    field: &Field,
    input_value: ParsedInputValue,
) -> QueryGraphBuilderResult<Vec<(DataSourceFieldRef, PrismaValue)>> {
    match field {
        Field::Scalar(sf) => {
            let value = input_value.try_into()?;
            Ok(vec![(sf.data_source_field().clone(), value)])
        }

        Field::Relation(rf) => {
            let dsfs = rf.data_source_fields();

            if dsfs.len() == 1 {
                let value = input_value.try_into()?;
                Ok(vec![(rf.data_source_fields().first().unwrap().clone(), value)])
            } else {
                let mut rf_map: ParsedInputMap = input_value.try_into()?;
                let mut pairs = vec![];

                for dsf in dsfs {
                    let pv: PrismaValue = rf_map.remove(&dsf.name).unwrap().try_into()?;
                    pairs.push((dsf.clone(), pv));
                }

                Ok(pairs)
            }
        }
    }
}

fn extract_compound_cursor_field(
    fields: Vec<Field>,
    input_value: ParsedInputValue,
) -> QueryGraphBuilderResult<Vec<(DataSourceFieldRef, PrismaValue)>> {
    let mut map: ParsedInputMap = input_value.try_into()?;
    let mut pairs = vec![];

    for field in fields {
        let value = map.remove(field.name()).unwrap();
        pairs.extend(extract_cursor_field(&field, value)?);
    }

    Ok(pairs)
}
