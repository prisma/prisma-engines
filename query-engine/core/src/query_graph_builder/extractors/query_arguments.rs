use super::*;
use crate::{
    constants::{aggregations, args, ordering},
    query_document::{ParsedArgument, ParsedInputMap},
    QueryGraphBuilderError, QueryGraphBuilderResult,
};
use connector::QueryArguments;
use prisma_models::prelude::*;
use std::convert::TryInto;

/// Expects the caller to know that it is structurally guaranteed that query arguments can be extracted,
/// e.g. that the query schema guarantees that required fields are present.
/// Errors occur if conversions fail.
#[tracing::instrument(skip(arguments, model))]
pub fn extract_query_args(arguments: Vec<ParsedArgument>, model: &ModelRef) -> QueryGraphBuilderResult<QueryArguments> {
    let query_args = arguments.into_iter().fold(
        Ok(QueryArguments::new(model.clone())),
        |result: QueryGraphBuilderResult<QueryArguments>, arg| {
            if let Ok(res) = result {
                match arg.name.as_str() {
                    args::CURSOR => Ok(QueryArguments {
                        cursor: extract_cursor(arg.value, model)?,
                        ..res
                    }),

                    args::TAKE => Ok(QueryArguments {
                        take: arg.value.try_into()?,
                        ..res
                    }),

                    args::SKIP => Ok(QueryArguments {
                        skip: extract_skip(arg.value)?,
                        ..res
                    }),

                    args::ORDER_BY => Ok(QueryArguments {
                        order_by: extract_order_by(&model.into(), arg.value)?,
                        ..res
                    }),

                    args::DISTINCT => Ok(QueryArguments {
                        distinct: Some(extract_distinct(arg.value)?),
                        ..res
                    }),

                    args::WHERE => {
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

/// Extracts order by conditions in order of appearance.
fn extract_order_by(container: &ParentContainer, value: ParsedInputValue) -> QueryGraphBuilderResult<Vec<OrderBy>> {
    match value {
        ParsedInputValue::List(list) => list
            .into_iter()
            .map(|list_value| {
                let object: ParsedInputMap = list_value.try_into()?;
                Ok(process_order_object(container, object, vec![], None)?)
            })
            .collect::<QueryGraphBuilderResult<Vec<_>>>()
            .map(|results| results.into_iter().flatten().collect()),

        ParsedInputValue::Map(map) => Ok(match process_order_object(container, map, vec![], None)? {
            Some(order) => vec![order],
            None => vec![],
        }),

        _ => unreachable!(),
    }
}

fn process_order_object(
    container: &ParentContainer,
    object: ParsedInputMap,
    mut path: Vec<OrderByHop>,
    parent_sort_aggregation: Option<SortAggregation>,
) -> QueryGraphBuilderResult<Option<OrderBy>> {
    match object.into_iter().next() {
        None => Ok(None),
        Some((field_name, field_value)) => {
            if field_name.as_str() == ordering::UNDERSCORE_RELEVANCE {
                let object: ParsedInputMap = field_value.try_into()?;

                return extract_order_by_relevance(container, object);
            }

            if let Some(sort_aggr) = extract_sort_aggregation(field_name.as_str()) {
                let object: ParsedInputMap = field_value.try_into()?;

                return process_order_object(container, object, path, Some(sort_aggr));
            }

            let field = container
                .find_field(&field_name)
                .expect("Fields must be valid after validation passed.");

            match field {
                Field::Relation(rf) if rf.is_list() => {
                    let object: ParsedInputMap = field_value.try_into()?;

                    path.push(rf.into());

                    let (inner_field_name, inner_field_value) = object.into_iter().next().unwrap();
                    let sort_aggregation = extract_sort_aggregation(inner_field_name.as_str())
                        .expect("To-many relation orderBy must be an aggregation ordering.");

                    let sort_order = extract_sort_order(inner_field_value)?;
                    Ok(Some(OrderBy::to_many_aggregation(path, sort_order, sort_aggregation)))
                }

                Field::Relation(rf) => {
                    let object: ParsedInputMap = field_value.try_into()?;
                    path.push((&rf).into());

                    process_order_object(&rf.related_model().into(), object, path, None)
                }

                Field::Scalar(sf) => {
                    let sort_order = extract_sort_order(field_value)?;

                    if let Some(sort_aggr) = parent_sort_aggregation {
                        // If the parent is a sort aggregation then this scalar is part of that one.
                        Ok(Some(OrderBy::scalar_aggregation(
                            sf.clone(),
                            vec![],
                            sort_order,
                            sort_aggr,
                        )))
                    } else {
                        Ok(Some(OrderBy::scalar(sf.clone(), path, sort_order)))
                    }
                }

                Field::Composite(cf) if cf.is_list() => {
                    let object: ParsedInputMap = field_value.try_into()?;

                    path.push(cf.into());

                    let (inner_field_name, inner_field_value) = object.into_iter().next().unwrap();
                    let sort_aggregation = extract_sort_aggregation(inner_field_name.as_str())
                        .expect("To-many composite orderBy must be an aggregation ordering.");

                    let sort_order = extract_sort_order(inner_field_value)?;
                    Ok(Some(OrderBy::to_many_aggregation(path, sort_order, sort_aggregation)))
                }

                Field::Composite(cf) => {
                    let object: ParsedInputMap = field_value.try_into()?;
                    path.push((&cf).into());

                    process_order_object(&cf.typ.clone().into(), object, path, None)
                }
            }
        }
    }
}

fn extract_order_by_relevance(
    container: &ParentContainer,
    object: ParsedInputMap,
) -> QueryGraphBuilderResult<Option<OrderBy>> {
    let sort_order = extract_sort_order(object.get(ordering::SORT).unwrap().clone())?;
    let search: PrismaValue = object.get(ordering::SEARCH).unwrap().clone().try_into()?;
    let search = search.into_string().unwrap();
    let fields: PrismaValue = object.get(ordering::FIELDS).unwrap().clone().try_into()?;

    let fields = match fields {
        PrismaValue::String(s) => Ok(vec![PrismaValue::String(s)]),
        PrismaValue::Enum(e) => Ok(vec![PrismaValue::String(e)]),
        PrismaValue::List(l) => Ok(l),
        x => Err(QueryGraphBuilderError::InputError(format!(
            "Expected field `fields` to be of type String, Enum or List<Enum>, found: {:?}",
            x
        ))),
    }?;

    let fields = fields
        .into_iter()
        .map(|pv| pv.into_string().unwrap())
        .map(|field_name| match container.find_field(&field_name) {
            Some(Field::Scalar(sf)) => Ok(sf),
            _ => Err(QueryGraphBuilderError::InputError(format!(
                "Invalid order-by reference input: Field {} is not a valid scalar field.",
                field_name
            ))),
        })
        .collect::<Result<Vec<ScalarFieldRef>, _>>()?;

    Ok(Some(OrderBy::relevance(fields, search, sort_order)))
}

fn extract_sort_aggregation(field_name: &str) -> Option<SortAggregation> {
    match field_name {
        aggregations::UNDERSCORE_COUNT => Some(SortAggregation::Count),
        aggregations::UNDERSCORE_AVG => Some(SortAggregation::Avg),
        aggregations::UNDERSCORE_SUM => Some(SortAggregation::Sum),
        aggregations::UNDERSCORE_MIN => Some(SortAggregation::Min),
        aggregations::UNDERSCORE_MAX => Some(SortAggregation::Max),
        _ => None,
    }
}

fn extract_sort_order(field_value: ParsedInputValue) -> QueryGraphBuilderResult<SortOrder> {
    let value: PrismaValue = field_value.try_into()?;
    let sort_order = match value.into_string().unwrap().to_lowercase().as_str() {
        ordering::ASC => SortOrder::Ascending,
        ordering::DESC => SortOrder::Descending,
        _ => unreachable!(),
    };

    Ok(sort_order)
}

fn extract_distinct(value: ParsedInputValue) -> QueryGraphBuilderResult<FieldSelection> {
    let selections = match value {
        ParsedInputValue::List(list) => list
            .into_iter()
            .map(|element| {
                let field: ScalarFieldRef = element.try_into()?;
                Ok(field.into())
            })
            .collect::<QueryGraphBuilderResult<Vec<_>>>()?,
        _ => unreachable!(),
    };

    Ok(FieldSelection::new(selections))
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

fn extract_cursor(value: ParsedInputValue, model: &ModelRef) -> QueryGraphBuilderResult<Option<SelectionResult>> {
    let input_map: ParsedInputMap = value.try_into()?;
    let mut pairs = vec![];

    for (field_name, map_value) in input_map {
        let additional_pairs = match model.fields().find_from_scalar(&field_name) {
            Ok(field) => extract_cursor_field(field, map_value)?,
            Err(_) => match utils::resolve_compound_field(&field_name, &model) {
                Some(fields) => extract_compound_cursor_field(fields, map_value)?,
                None => {
                    return Err(QueryGraphBuilderError::AssertionError(format!(
                        "Unable to resolve field {} to a field or a set of fields on model {}",
                        field_name, model.name
                    )))
                }
            },
        };

        pairs.extend(additional_pairs);
    }

    Ok(Some(SelectionResult::new(pairs)))
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
        (args.skip.as_ref().map(|skip| *skip > 0).unwrap_or(false) || args.cursor.is_some() || args.take.is_some())
            && args.order_by.is_empty();

    if add_implicit_ordering {
        let primary_identifier = model.primary_identifier();
        let order_bys = primary_identifier.into_iter().map(|f| match f {
            // IDs can _only_ contain scalar selections.
            SelectedField::Scalar(sf) => sf.into(),
            _ => unreachable!(),
        });

        args.order_by.extend(order_bys);
    }

    args
}
