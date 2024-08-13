use super::*;
use crate::{
    query_document::{ParsedArgument, ParsedInputMap},
    QueryGraphBuilderError, QueryGraphBuilderResult,
};
use query_structure::{prelude::*, QueryArguments};
use schema::constants::{aggregations, args, ordering};
use std::convert::TryInto;

/// Expects the caller to know that it is structurally guaranteed that query arguments can be extracted,
/// e.g. that the query schema guarantees that required fields are present.
/// Errors occur if conversions fail.
pub fn extract_query_args(
    arguments: Vec<ParsedArgument<'_>>,
    model: &Model,
) -> QueryGraphBuilderResult<QueryArguments> {
    let query_args = arguments.into_iter().try_fold(
        QueryArguments::new(model.clone()),
        |result, arg| -> QueryGraphBuilderResult<QueryArguments> {
            match arg.name.as_str() {
                args::CURSOR => Ok(QueryArguments {
                    cursor: extract_cursor(arg.value, model)?,
                    ..result
                }),

                args::TAKE => Ok(QueryArguments {
                    take: arg.value.try_into()?,
                    ..result
                }),

                args::SKIP => Ok(QueryArguments {
                    skip: extract_skip(arg.value)?,
                    ..result
                }),

                args::ORDER_BY => Ok(QueryArguments {
                    order_by: extract_order_by(&model.into(), arg.value)?,
                    ..result
                }),

                args::DISTINCT => Ok(QueryArguments {
                    distinct: Some(extract_distinct(arg.value)?),
                    ..result
                }),

                args::WHERE => {
                    let val: Option<ParsedInputMap<'_>> = arg.value.try_into()?;
                    match val {
                        Some(m) => {
                            let filter = Some(extract_filter(m, model)?);
                            Ok(QueryArguments { filter, ..result })
                        }
                        None => Ok(result),
                    }
                }

                args::RELATION_LOAD_STRATEGY => Ok(QueryArguments {
                    relation_load_strategy: Some(arg.value.try_into()?),
                    ..result
                }),

                _ => Ok(result),
            }
        },
    )?;

    Ok(finalize_arguments(query_args, model))
}

/// Extracts order by conditions in order of appearance.
fn extract_order_by(container: &ParentContainer, value: ParsedInputValue<'_>) -> QueryGraphBuilderResult<Vec<OrderBy>> {
    match value {
        ParsedInputValue::List(list) => list
            .into_iter()
            .map(|list_value| {
                let object: ParsedInputMap<'_> = list_value.try_into()?;
                process_order_object(container, object, vec![], None)
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
    object: ParsedInputMap<'_>,
    mut path: Vec<OrderByHop>,
    parent_sort_aggregation: Option<SortAggregation>,
) -> QueryGraphBuilderResult<Option<OrderBy>> {
    match object.into_iter().next() {
        None => Ok(None),
        Some((field_name, field_value)) => {
            if field_name.as_ref() == ordering::UNDERSCORE_RELEVANCE {
                let object: ParsedInputMap<'_> = field_value.try_into()?;

                return extract_order_by_relevance(container, object, path);
            }

            if let Some(sort_aggr) = extract_sort_aggregation(field_name.as_ref()) {
                let object: ParsedInputMap<'_> = field_value.try_into()?;

                return process_order_object(container, object, path, Some(sort_aggr));
            }

            let field = container
                .find_field(&field_name)
                .expect("Fields must be valid after validation passed.");

            match field {
                Field::Relation(rf) if rf.is_list() => {
                    let object: ParsedInputMap<'_> = field_value.try_into()?;

                    path.push(rf.into());

                    let (inner_field_name, inner_field_value) = object.into_iter().next().unwrap();
                    let sort_aggregation = extract_sort_aggregation(inner_field_name.as_ref())
                        .expect("To-many relation orderBy must be an aggregation ordering.");

                    let (sort_order, _) = extract_order_by_args(inner_field_value)?;
                    Ok(Some(OrderBy::to_many_aggregation(path, sort_order, sort_aggregation)))
                }

                Field::Relation(rf) => {
                    let object: ParsedInputMap<'_> = field_value.try_into()?;
                    path.push((&rf).into());

                    process_order_object(&rf.related_model().into(), object, path, None)
                }

                Field::Scalar(sf) => {
                    let (sort_order, nulls_order) = extract_order_by_args(field_value)?;

                    if let Some(sort_aggr) = parent_sort_aggregation {
                        // If the parent is a sort aggregation then this scalar is part of that one.
                        Ok(Some(OrderBy::scalar_aggregation(sf, sort_order, sort_aggr)))
                    } else {
                        Ok(Some(OrderBy::scalar(sf, path, sort_order, nulls_order)))
                    }
                }

                Field::Composite(cf) if cf.is_list() => {
                    let object: ParsedInputMap<'_> = field_value.try_into()?;

                    path.push(cf.into());

                    let (inner_field_name, inner_field_value) = object.into_iter().next().unwrap();
                    let sort_aggregation = extract_sort_aggregation(inner_field_name.as_ref())
                        .expect("To-many composite orderBy must be an aggregation ordering.");

                    let (sort_order, _) = extract_order_by_args(inner_field_value)?;
                    Ok(Some(OrderBy::to_many_aggregation(path, sort_order, sort_aggregation)))
                }

                Field::Composite(cf) => {
                    let object: ParsedInputMap<'_> = field_value.try_into()?;
                    path.push((&cf).into());

                    process_order_object(&cf.typ().into(), object, path, None)
                }
            }
        }
    }
}

fn extract_order_by_relevance(
    container: &ParentContainer,
    object: ParsedInputMap<'_>,
    path: Vec<OrderByHop>,
) -> QueryGraphBuilderResult<Option<OrderBy>> {
    let (sort_order, _) = extract_order_by_args(object.get(ordering::SORT).unwrap().clone())?;
    let search: PrismaValue = object.get(ordering::SEARCH).unwrap().clone().try_into()?;
    let search = search.into_string().unwrap();
    let fields: PrismaValue = object.get(ordering::FIELDS).unwrap().clone().try_into()?;

    let fields = match fields {
        PrismaValue::String(s) => Ok(vec![PrismaValue::String(s)]),
        PrismaValue::Enum(e) => Ok(vec![PrismaValue::String(e)]),
        PrismaValue::List(l) => Ok(l),
        x => Err(QueryGraphBuilderError::InputError(format!(
            "Expected field `fields` to be of type String, Enum or List<Enum>, found: {x:?}"
        ))),
    }?;

    let fields = fields
        .into_iter()
        .map(|pv| pv.into_string().unwrap())
        .map(|field_name| match container.find_field(&field_name) {
            Some(Field::Scalar(sf)) => Ok(sf),
            _ => Err(QueryGraphBuilderError::InputError(format!(
                "Invalid order-by reference input: Field {field_name} is not a valid scalar field."
            ))),
        })
        .collect::<Result<Vec<ScalarFieldRef>, _>>()?;

    Ok(Some(OrderBy::relevance(fields, search, sort_order, path)))
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

fn extract_order_by_args(
    field_value: ParsedInputValue<'_>,
) -> QueryGraphBuilderResult<(SortOrder, Option<NullsOrder>)> {
    match field_value {
        ParsedInputValue::Map(mut map) => {
            let sort: PrismaValue = map.swap_remove(ordering::SORT).unwrap().try_into()?;
            let sort = pv_to_sort_order(sort)?;
            let nulls = map
                .swap_remove(ordering::NULLS)
                .map(PrismaValue::try_from)
                .transpose()?
                .map(pv_to_nulls_order)
                .transpose()?;

            Ok((sort, nulls))
        }
        ParsedInputValue::Single(pv) => Ok((pv_to_sort_order(pv)?, None)),
        _ => unreachable!(),
    }
}

fn pv_to_sort_order(pv: PrismaValue) -> QueryGraphBuilderResult<SortOrder> {
    let sort_order = match pv.into_string().unwrap().as_str() {
        ordering::ASC => SortOrder::Ascending,
        ordering::DESC => SortOrder::Descending,
        _ => unreachable!(),
    };

    Ok(sort_order)
}

fn pv_to_nulls_order(pv: PrismaValue) -> QueryGraphBuilderResult<NullsOrder> {
    let nulls_order = match pv.into_string().unwrap().as_str() {
        ordering::FIRST => NullsOrder::First,
        ordering::LAST => NullsOrder::Last,
        _ => unreachable!(),
    };

    Ok(nulls_order)
}

fn extract_distinct(value: ParsedInputValue<'_>) -> QueryGraphBuilderResult<FieldSelection> {
    let selections = match value {
        ParsedInputValue::List(list) => list
            .into_iter()
            .map(|element| {
                let field: ScalarFieldRef = element.try_into()?;
                Ok(field.into())
            })
            .collect::<QueryGraphBuilderResult<Vec<_>>>()?,
        ParsedInputValue::ScalarField(sf) => {
            vec![sf.into()]
        }
        _ => unreachable!(),
    };

    Ok(FieldSelection::new(selections))
}

fn extract_skip(value: ParsedInputValue<'_>) -> QueryGraphBuilderResult<Option<i64>> {
    let val: Option<i64> = value.try_into()?;

    match val {
        Some(val) if val < 0 => Err(QueryGraphBuilderError::AssertionError(format!(
            "Invalid value for skip argument: Value can only be positive, found: {val}",
        ))),

        val => Ok(val),
    }
}

fn extract_cursor(value: ParsedInputValue<'_>, model: &Model) -> QueryGraphBuilderResult<Option<SelectionResult>> {
    let input_map: ParsedInputMap<'_> = value.try_into()?;
    let mut pairs = vec![];

    for (field_name, map_value) in input_map {
        let additional_pairs = match model.fields().find_from_scalar(&field_name) {
            Ok(field) => extract_cursor_field(field, map_value)?,
            Err(_) => match utils::resolve_compound_field(&field_name, model) {
                Some(fields) => extract_compound_cursor_field(fields, map_value)?,
                None => {
                    return Err(QueryGraphBuilderError::AssertionError(format!(
                        "Unable to resolve field {} to a field or a set of fields on model {}",
                        field_name,
                        model.name()
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
    input_value: ParsedInputValue<'_>,
) -> QueryGraphBuilderResult<Vec<(ScalarFieldRef, PrismaValue)>> {
    let value = input_value.try_into()?;
    Ok(vec![(field, value)])
}

fn extract_compound_cursor_field(
    fields: Vec<ScalarFieldRef>,
    input_value: ParsedInputValue<'_>,
) -> QueryGraphBuilderResult<Vec<(ScalarFieldRef, PrismaValue)>> {
    let mut map: ParsedInputMap<'_> = input_value.try_into()?;
    let mut pairs = vec![];

    for field in fields {
        let value = map.swap_remove(field.name()).unwrap();
        pairs.extend(extract_cursor_field(field, value)?);
    }

    Ok(pairs)
}

/// Runs final transformations on the QueryArguments.
fn finalize_arguments(mut args: QueryArguments, model: &Model) -> QueryArguments {
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
