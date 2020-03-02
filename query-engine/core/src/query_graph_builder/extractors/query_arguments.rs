use super::*;
use crate::{
    query_document::{ParsedArgument, ParsedInputMap},
    QueryGraphBuilderError, QueryGraphBuilderResult,
};
use connector::QueryArguments;
use prisma_models::{Field, ModelRef, PrismaValue, RecordProjection};
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

fn extract_cursor(value: ParsedInputValue, model: &ModelRef) -> QueryGraphBuilderResult<Option<RecordProjection>> {
    if let Err(_) = value.assert_non_null() {
        return Ok(None);
    }

    let map: ParsedInputMap = value.try_into()?;

    // map.assert_size(1)?;
    // let (field_name, value): (String, ParsedInputValue) = map.into_iter().nth(0).unwrap();

    // Always try to resolve regular fields first. If that fails, try to resolve compound fields.
    model
        .fields()
        .find_from_all(&field_name)
        .map_err(|err| err.into())
        .and_then(|field| {
            match field {
                Field::Scalar(sf) => {
                    let value: PrismaValue = value.try_into()?;

                    Ok(Some(RecordProjection::new(vec![(
                        sf.data_source_field().clone(),
                        value,
                    )])))
                }

                Field::Relation(rf) => {
                    let fields = rf.data_source_fields();

                    if fields.len() == 1 {
                        let value: PrismaValue = value.try_into()?;

                        Ok(Some(RecordProjection::new(vec![(
                            fields.first().unwrap().clone(),
                            value,
                        )])))
                    } else {
                        let mut map: ParsedInputMap = value.try_into()?;

                        let pairs: Vec<_> = fields
                            .into_iter()
                            .map(|field| {
                                // Every field in the map must correspond to
                                // If a field is not present, nulls are inserted.

                                map.remove()

                                todo!()
                            })
                            .collect();

                        Ok(Some(RecordProjection::new(pairs)))
                    }
                }
            }
        })
        .or_else(|_: QueryGraphBuilderError| {
            // utils::resolve_compound_field(&field_name, &model)
            //     .ok_or(QueryGraphBuilderError::AssertionError(format!(
            //         "Unable to resolve field {} to a field or a set of fields on model {}",
            //         field_name, model.name
            //     )))
            //     .and_then(|fields| {
            //         let mut compound_map: ParsedInputMap = value.try_into()?;
            //         let mut result = vec![];

            //         for field in fields {
            //             // Unwrap is safe because validation gurantees that the value is present.
            //             let value = compound_map.remove(&field.name).unwrap().try_into()?;
            //             result.push((field, value));
            //         }

            //         Ok(Some(result))
            //     })

            todo!()
        })
}
