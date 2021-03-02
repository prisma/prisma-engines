use super::*;
use crate::{
    constants::inputs::args,
    constants::inputs::filters,
    constants::inputs::ordering,
    query_document::{ParsedArgument, ParsedInputMap},
    QueryGraphBuilderError, QueryGraphBuilderResult,
};
use connector::QueryArguments;
use prisma_models::{
    Field, ModelProjection, ModelRef, OrderBy, PrismaValue, RecordProjection, RelationFieldRef, ScalarFieldRef,
    SortAggregation, SortOrder,
};
use std::convert::{identity, TryInto};

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
                        order_by: extract_order_by(model, arg.value)?,
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
fn extract_order_by(model: &ModelRef, value: ParsedInputValue) -> QueryGraphBuilderResult<Vec<OrderBy>> {
    match value {
        ParsedInputValue::List(list) => list
            .into_iter()
            .map(|list_value| {
                let object: ParsedInputMap = list_value.try_into()?;
                Ok(process_order_object(model, object, vec![])?)
            })
            .collect::<QueryGraphBuilderResult<Vec<_>>>()
            .map(|results| results.into_iter().filter_map(identity).collect()),

        ParsedInputValue::Map(map) => Ok(match process_order_object(model, map, vec![])? {
            Some(order) => vec![order],
            None => vec![],
        }),

        _ => unreachable!(),
    }
}

#[tracing::instrument(skip(model, object, path))]
fn process_order_object(
    model: &ModelRef,
    object: ParsedInputMap,
    mut path: Vec<RelationFieldRef>,
) -> QueryGraphBuilderResult<Option<OrderBy>> {
    match object.into_iter().next() {
        None => Ok(None),
        Some((field_name, field_value)) => {
            let field = model.fields().find_from_all(&field_name)?;
            match field {
                Field::Relation(rf) if rf.is_list => {
                    path.push(rf.clone());

                    let object: ParsedInputMap = field_value.try_into()?;
                    let (sort_aggregation, sort_order) = extract_sort_aggregation(object)?;
                    let ids: Vec<_> = rf.related_model().primary_identifier().scalar_fields().collect();
                    // FIXME: This is a hack to fulfil the requirement of the `OrderBy` struct to have a field to order by
                    // In the case of aggregations, at least for now, we use AGGR(*), meaning that this field won't ever be used
                    // This needs to be refactored when we add order by aggregations on specific fields
                    let first_id = ids.first().unwrap();

                    Ok(Some(OrderBy::new(
                        first_id.clone(),
                        path,
                        sort_order,
                        Some(sort_aggregation),
                    )))
                }
                Field::Relation(rf) => {
                    path.push(rf.clone());

                    let object: ParsedInputMap = field_value.try_into()?;
                    process_order_object(&rf.related_model(), object, path)
                }
                Field::Scalar(sf) => {
                    let sort_order = extract_sort_order(field_value)?;

                    Ok(Some(OrderBy::new(sf.clone(), path, sort_order, None)))
                }
            }
        }
    }
}

fn extract_sort_aggregation(object: ParsedInputMap) -> QueryGraphBuilderResult<(SortAggregation, SortOrder)> {
    let (field_name, field_value) = object.into_iter().next().unwrap();
    let sort_order = extract_sort_order(field_value)?;
    let sort_aggregation = match field_name.as_str() {
        filters::COUNT => Some(SortAggregation::Count { _all: true }),
        _ => unreachable!("No aggregation operation could be found. This should not happen"),
    };

    Ok((sort_aggregation.unwrap(), sort_order))
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
        (args.skip.as_ref().map(|skip| *skip > 0).unwrap_or(false) || args.cursor.is_some() || args.take.is_some())
            && args.order_by.is_empty();

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
