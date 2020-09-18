use super::*;
use crate::{
    query_document::{ParsedArgument, ParsedInputMap},
    QueryGraphBuilderError, QueryGraphBuilderResult,
};
use connector::QueryArguments;
use prisma_models::{
    Field, ModelProjection, ModelRef, OrderBy, PrismaValue, RecordProjection, ScalarFieldRef, SortOrder,
};
use std::convert::{identity, TryInto};

/// Expects the caller to know that it is structurally guaranteed that query arguments can be extracted,
/// e.g. that the query schema guarantees that required fields are present.
/// Errors occur if conversions fail.
pub fn extract_query_args(arguments: Vec<ParsedArgument>, model: &ModelRef) -> QueryGraphBuilderResult<QueryArguments> {
    let query_args = arguments.into_iter().fold(
        Ok(QueryArguments::new(model.clone())),
        |result: QueryGraphBuilderResult<QueryArguments>, arg| {
            if let Ok(res) = result {
                match arg.name.as_str() {
                    "cursor" => Ok(QueryArguments {
                        cursor: extract_cursor(arg.value, model)?,
                        ..res
                    }),

                    "take" => Ok(QueryArguments {
                        take: arg.value.try_into()?,
                        ..res
                    }),

                    "skip" => Ok(QueryArguments {
                        skip: extract_skip(arg.value)?,
                        ..res
                    }),

                    "orderBy" => Ok(QueryArguments {
                        order_by: extract_order_by(model, arg.value)?,
                        ..res
                    }),

                    "distinct" => Ok(QueryArguments {
                        distinct: Some(extract_distinct(arg.value)?),
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
        },
    )?;

    Ok(finalize_arguments(query_args, model))
}

/// Extracts order by conditions in order of appearance, as defined in
fn extract_order_by(model: &ModelRef, value: ParsedInputValue) -> QueryGraphBuilderResult<Vec<OrderBy>> {
    match value {
        ParsedInputValue::List(list) => list
            .into_iter()
            .map(|list_value| {
                let object: ParsedInputMap = list_value.try_into()?;

                match object.into_iter().next() {
                    None => Ok(None),
                    Some((field_name, sort_order)) => {
                        let field = model.fields().find_from_scalar(&field_name)?;
                        let value: PrismaValue = sort_order.try_into()?;
                        let sort_order = match value.into_string().unwrap().to_lowercase().as_str() {
                            "asc" => SortOrder::Ascending,
                            "desc" => SortOrder::Descending,
                            _ => unreachable!(),
                        };

                        Ok(Some(OrderBy::new(field, sort_order)))
                    }
                }
            })
            .collect::<QueryGraphBuilderResult<Vec<_>>>()
            .map(|results| results.into_iter().filter_map(identity).collect()),

        ParsedInputValue::Map(map) => Ok(match process_order_object(model, map)? {
            Some(order) => vec![order],
            None => vec![],
        }),

        _ => unreachable!(),
    }
}

fn process_order_object(model: &ModelRef, object: ParsedInputMap) -> QueryGraphBuilderResult<Option<OrderBy>> {
    // let object: ParsedInputMap = list_value.try_into()?;

    match object.into_iter().next() {
        None => Ok(None),
        Some((field_name, sort_order)) => {
            let field = model.fields().find_from_scalar(&field_name)?;
            let value: PrismaValue = sort_order.try_into()?;
            let sort_order = match value.into_string().unwrap().to_lowercase().as_str() {
                "asc" => SortOrder::Ascending,
                "desc" => SortOrder::Descending,
                _ => unreachable!(),
            };

            Ok(Some(OrderBy::new(field, sort_order)))
        }
    }
}

fn extract_distinct(value: ParsedInputValue) -> QueryGraphBuilderResult<ModelProjection> {
    let fields: Vec<Field> = match value {
        ParsedInputValue::List(list) => list
            .into_iter()
            .map(|element| {
                let field: ScalarFieldRef = element.try_into()?;
                Ok(field.into())
            })
            .collect::<QueryGraphBuilderResult<Vec<_>>>()?,
        _ => unreachable!(),
    };

    Ok(ModelProjection::new(fields))
}

fn extract_skip(value: ParsedInputValue) -> QueryGraphBuilderResult<Option<i64>> {
    let val: Option<i64> = value.try_into()?;

    match val {
        Some(val) if val < 0 => Err(QueryGraphBuilderError::AssertionError(format!(
            "Invalid value for skip argument: Value can only be positive, found: {}",
            val,
        ))),

        val => Ok(val),
    }
}

fn extract_cursor(value: ParsedInputValue, model: &ModelRef) -> QueryGraphBuilderResult<Option<RecordProjection>> {
    let input_map: ParsedInputMap = value.try_into()?;
    let mut pairs = vec![];

    for (field_name, map_value) in input_map {
        let additional_pairs = match model.fields().find_from_scalar(&field_name) {
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
    field: ScalarFieldRef,
    input_value: ParsedInputValue,
) -> QueryGraphBuilderResult<Vec<(ScalarFieldRef, PrismaValue)>> {
    let value = input_value.try_into()?;
    Ok(vec![(field, value)])
}

fn extract_compound_cursor_field(
    fields: Vec<ScalarFieldRef>,
    input_value: ParsedInputValue,
) -> QueryGraphBuilderResult<Vec<(ScalarFieldRef, PrismaValue)>> {
    let mut map: ParsedInputMap = input_value.try_into()?;
    let mut pairs = vec![];

    for field in fields {
        let value = map.remove(&field.name).unwrap();
        pairs.extend(extract_cursor_field(field, value)?);
    }

    Ok(pairs)
}

/// Runs final transformations on the QueryArguments.
fn finalize_arguments(mut args: QueryArguments, model: &ModelRef) -> QueryArguments {
    // Check if the query requires an implicit ordering added to the arguments.
    // An implicit ordering is convenient for deterministic results for take and skip, for cursor it's _required_
    // as a cursor needs a direction to page. We simply take the primary identifier as a default order-by.
    let add_implicit_ordering =
        (args.skip.is_some() || args.cursor.is_some() || args.take.is_some()) && args.order_by.is_empty();

    if add_implicit_ordering {
        let primary_identifier = model.primary_identifier();
        let order_bys = primary_identifier.into_iter().map(|f| match f {
            Field::Scalar(f) => f.into(),
            _ => unreachable!(),
        });

        args.order_by.extend(order_bys);
    }

    args
}
