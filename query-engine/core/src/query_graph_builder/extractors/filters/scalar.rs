use crate::{
    constants::{aggregations, filters},
    ParsedInputMap, ParsedInputValue, QueryGraphBuilderError, QueryGraphBuilderResult,
};
use connector::{Filter, JsonCompare, JsonFilterPath, JsonTargetType, ScalarCompare, ScalarListCompare};
use prisma_models::{PrismaValue, ScalarFieldRef, TypeIdentifier};
use std::convert::TryInto;

#[tracing::instrument(name = "parse_scalar_field", skip(input_map, reverse))]
pub fn parse(
    mut input_map: ParsedInputMap,
    field: &ScalarFieldRef,
    reverse: bool,
) -> QueryGraphBuilderResult<Vec<Filter>> {
    let json_path: Option<JsonFilterPath> = match input_map.remove(filters::PATH) {
        Some(v) => Some(parse_json_path(v)?),
        _ => None,
    };

    let filters: Vec<Filter> = input_map
        .into_iter()
        .map(|(k, v)| match field.type_identifier {
            TypeIdentifier::Json => parse_internal_json(&k, v, field, &json_path, reverse),
            _ => parse_internal_scalar(&k, v, field, reverse),
        })
        .collect::<QueryGraphBuilderResult<Vec<Vec<_>>>>()?
        .into_iter()
        .flatten()
        .collect();

    if json_path.is_some() && filters.is_empty() {
        return Err(QueryGraphBuilderError::InputError(
            "A JSON path cannot be set without a scalar filter.".to_owned(),
        ));
    }

    Ok(filters)
}

fn parse_internal_json(
    filter_key: &str,
    input: ParsedInputValue,
    field: &ScalarFieldRef,
    json_path: &Option<JsonFilterPath>,
    reverse: bool,
) -> QueryGraphBuilderResult<Vec<Filter>> {
    match filter_key {
        filters::NOT_LOWERCASE => {
            match input {
                // Support for syntax `{ scalarField: { not: null } }` and `{ scalarField: { not: <value> } }`
                ParsedInputValue::Single(value) => Ok(vec![field.json_not_equals(value, json_path.to_owned())]),
                _ => {
                    let inner_object: ParsedInputMap = input.try_into()?;

                    parse(inner_object, field, !reverse)
                }
            }
        }
        filters::EQUALS if reverse => Ok(vec![
            field.json_not_equals(as_prisma_value(input)?, json_path.to_owned())
        ]),
        filters::EQUALS => Ok(vec![field.json_equals(as_prisma_value(input)?, json_path.to_owned())]),
        filters::LOWER_THAN if reverse => {
            Ok(vec![field.json_greater_than_or_equals(
                as_prisma_value(input)?,
                json_path.to_owned(),
            )])
        }
        filters::GREATER_THAN if reverse => {
            Ok(vec![field.json_less_than_or_equals(
                as_prisma_value(input)?,
                json_path.to_owned(),
            )])
        }
        filters::LOWER_THAN_OR_EQUAL if reverse => Ok(vec![
            field.json_greater_than(as_prisma_value(input)?, json_path.to_owned())
        ]),
        filters::GREATER_THAN_OR_EQUAL if reverse => {
            Ok(vec![field.json_less_than(as_prisma_value(input)?, json_path.to_owned())])
        }

        filters::LOWER_THAN => Ok(vec![field.json_less_than(as_prisma_value(input)?, json_path.to_owned())]),
        filters::GREATER_THAN => Ok(vec![
            field.json_greater_than(as_prisma_value(input)?, json_path.to_owned())
        ]),
        filters::LOWER_THAN_OR_EQUAL => {
            Ok(vec![field.json_less_than_or_equals(
                as_prisma_value(input)?,
                json_path.to_owned(),
            )])
        }
        filters::GREATER_THAN_OR_EQUAL => {
            Ok(vec![field.json_greater_than_or_equals(
                as_prisma_value(input)?,
                json_path.to_owned(),
            )])
        }

        // List-specific filters
        filters::HAS => Ok(vec![field.contains_element(as_prisma_value(input)?)]),
        filters::HAS_EVERY => Ok(vec![field.contains_every_element(as_prisma_value_list(input)?)]),
        filters::HAS_SOME => Ok(vec![field.contains_some_element(as_prisma_value_list(input)?)]),
        filters::IS_EMPTY => Ok(vec![field.is_empty_list(input.try_into()?)]),

        // Json-specific filters
        filters::ARRAY_CONTAINS if reverse => Ok(vec![field.json_not_contains(
            as_prisma_value(input)?,
            json_path.to_owned(),
            JsonTargetType::Array,
        )]),
        filters::ARRAY_STARTS_WITH if reverse => Ok(vec![field.json_not_starts_with(
            as_prisma_value(input)?,
            json_path.to_owned(),
            JsonTargetType::Array,
        )]),
        filters::ARRAY_ENDS_WITH if reverse => Ok(vec![field.json_not_ends_with(
            as_prisma_value(input)?,
            json_path.to_owned(),
            JsonTargetType::Array,
        )]),
        filters::STRING_CONTAINS if reverse => Ok(vec![field.json_not_contains(
            as_prisma_value(input)?,
            json_path.to_owned(),
            JsonTargetType::String,
        )]),
        filters::STRING_STARTS_WITH if reverse => Ok(vec![field.json_not_starts_with(
            as_prisma_value(input)?,
            json_path.to_owned(),
            JsonTargetType::String,
        )]),
        filters::STRING_ENDS_WITH if reverse => Ok(vec![field.json_not_ends_with(
            as_prisma_value(input)?,
            json_path.to_owned(),
            JsonTargetType::String,
        )]),
        filters::ARRAY_CONTAINS => Ok(vec![field.json_contains(
            as_prisma_value(input)?,
            json_path.to_owned(),
            JsonTargetType::Array,
        )]),
        filters::ARRAY_STARTS_WITH => Ok(vec![field.json_starts_with(
            as_prisma_value(input)?,
            json_path.to_owned(),
            JsonTargetType::Array,
        )]),
        filters::ARRAY_ENDS_WITH => Ok(vec![field.json_ends_with(
            as_prisma_value(input)?,
            json_path.to_owned(),
            JsonTargetType::Array,
        )]),
        filters::STRING_CONTAINS => Ok(vec![field.json_contains(
            as_prisma_value(input)?,
            json_path.to_owned(),
            JsonTargetType::String,
        )]),
        filters::STRING_STARTS_WITH => Ok(vec![field.json_starts_with(
            as_prisma_value(input)?,
            json_path.to_owned(),
            JsonTargetType::String,
        )]),
        filters::STRING_ENDS_WITH => Ok(vec![field.json_ends_with(
            as_prisma_value(input)?,
            json_path.to_owned(),
            JsonTargetType::String,
        )]),
        _ => {
            return Err(QueryGraphBuilderError::InputError(format!(
                "{} is not a valid scalar filter operation",
                filter_key
            )))
        }
    }
}

fn parse_internal_scalar(
    filter_key: &str,
    input: ParsedInputValue,
    field: &ScalarFieldRef,
    reverse: bool,
) -> QueryGraphBuilderResult<Vec<Filter>> {
    match filter_key {
        filters::NOT_LOWERCASE => {
            match input {
                // Support for syntax `{ scalarField: { not: null } }` and `{ scalarField: { not: <value> } }`
                ParsedInputValue::Single(value) => Ok(vec![field.not_equals(value)]),
                _ => {
                    let inner_object: ParsedInputMap = input.try_into()?;

                    parse(inner_object, field, !reverse)
                }
            }
        }

        filters::IN => {
            let value: PrismaValue = input.try_into()?;
            let filter = match value {
                PrismaValue::Null if reverse => field.not_equals(value),
                PrismaValue::List(values) if reverse => field.not_in(values),

                PrismaValue::Null => field.equals(value),
                PrismaValue::List(values) => field.is_in(values),

                _ => unreachable!(), // Validation guarantees this.
            };

            Ok(vec![filter])
        }

        // Legacy operation
        filters::NOT_IN => {
            let value: PrismaValue = input.try_into()?;
            let filter = match value {
                PrismaValue::Null if reverse => field.equals(value), // not not in null => in null
                PrismaValue::List(values) if reverse => field.is_in(values), // not not in values => in values

                PrismaValue::Null => field.not_equals(value),
                PrismaValue::List(values) => field.not_in(values),

                _ => unreachable!(), // Validation guarantees this.
            };

            Ok(vec![filter])
        }

        filters::EQUALS if reverse => Ok(vec![field.not_equals(as_prisma_value(input)?)]),
        filters::CONTAINS if reverse => Ok(vec![field.not_contains(as_prisma_value(input)?)]),
        filters::STARTS_WITH if reverse => Ok(vec![field.not_starts_with(as_prisma_value(input)?)]),
        filters::ENDS_WITH if reverse => Ok(vec![field.not_ends_with(as_prisma_value(input)?)]),

        filters::EQUALS => Ok(vec![field.equals(as_prisma_value(input)?)]),
        filters::CONTAINS => Ok(vec![field.contains(as_prisma_value(input)?)]),
        filters::STARTS_WITH => Ok(vec![field.starts_with(as_prisma_value(input)?)]),
        filters::ENDS_WITH => Ok(vec![field.ends_with(as_prisma_value(input)?)]),

        filters::LOWER_THAN if reverse => Ok(vec![field.greater_than_or_equals(as_prisma_value(input)?)]),
        filters::GREATER_THAN if reverse => Ok(vec![field.less_than_or_equals(as_prisma_value(input)?)]),
        filters::LOWER_THAN_OR_EQUAL if reverse => Ok(vec![field.greater_than(as_prisma_value(input)?)]),
        filters::GREATER_THAN_OR_EQUAL if reverse => Ok(vec![field.less_than(as_prisma_value(input)?)]),

        filters::LOWER_THAN => Ok(vec![field.less_than(as_prisma_value(input)?)]),
        filters::GREATER_THAN => Ok(vec![field.greater_than(as_prisma_value(input)?)]),
        filters::LOWER_THAN_OR_EQUAL => Ok(vec![field.less_than_or_equals(as_prisma_value(input)?)]),
        filters::GREATER_THAN_OR_EQUAL => Ok(vec![field.greater_than_or_equals(as_prisma_value(input)?)]),

        filters::SEARCH if reverse => Ok(vec![field.search(as_prisma_value(input)?)]),
        filters::SEARCH => Ok(vec![field.not_search(as_prisma_value(input)?)]),

        // List-specific filters
        filters::HAS => Ok(vec![field.contains_element(as_prisma_value(input)?)]),
        filters::HAS_EVERY => Ok(vec![field.contains_every_element(as_prisma_value_list(input)?)]),
        filters::HAS_SOME => Ok(vec![field.contains_some_element(as_prisma_value_list(input)?)]),
        filters::IS_EMPTY => Ok(vec![field.is_empty_list(input.try_into()?)]),

        // Aggregation filters
        aggregations::UNDERSCORE_COUNT => aggregation_filter(field, input, reverse, Filter::count),
        aggregations::UNDERSCORE_AVG => aggregation_filter(field, input, reverse, Filter::average),
        aggregations::UNDERSCORE_SUM => aggregation_filter(field, input, reverse, Filter::sum),
        aggregations::UNDERSCORE_MIN => aggregation_filter(field, input, reverse, Filter::min),
        aggregations::UNDERSCORE_MAX => aggregation_filter(field, input, reverse, Filter::max),

        // Deprecated aggregation filters
        aggregations::COUNT => aggregation_filter(field, input, reverse, Filter::count),
        aggregations::AVG => aggregation_filter(field, input, reverse, Filter::average),
        aggregations::SUM => aggregation_filter(field, input, reverse, Filter::sum),
        aggregations::MIN => aggregation_filter(field, input, reverse, Filter::min),
        aggregations::MAX => aggregation_filter(field, input, reverse, Filter::max),

        _ => {
            return Err(QueryGraphBuilderError::InputError(format!(
                "{} is not a valid scalar filter operation",
                filter_key
            )))
        }
    }
}

fn as_prisma_value(input: ParsedInputValue) -> QueryGraphBuilderResult<PrismaValue> {
    Ok(input.try_into()?)
}

fn as_prisma_value_list(input: ParsedInputValue) -> QueryGraphBuilderResult<Vec<PrismaValue>> {
    Ok(input.try_into()?)
}

fn aggregation_filter<F>(
    field: &ScalarFieldRef,
    input: ParsedInputValue,
    reverse: bool,
    func: F,
) -> QueryGraphBuilderResult<Vec<Filter>>
where
    F: Fn(Filter) -> Filter,
{
    let inner_object: ParsedInputMap = input.try_into()?;
    let filters: Vec<Filter> = parse(inner_object, field, reverse)?;

    Ok(filters.into_iter().map(|filter| func(filter)).collect())
}

fn parse_json_path(input: ParsedInputValue) -> QueryGraphBuilderResult<JsonFilterPath> {
    let path: PrismaValue = input.try_into()?;

    match path {
        PrismaValue::String(str) => Ok(JsonFilterPath::String(str)),
        PrismaValue::List(list) => {
            let keys = list
                .into_iter()
                .map(|key| {
                    key.into_string()
                        .expect("Json filtering array path elements must all be of type string")
                })
                .collect();

            Ok(JsonFilterPath::Array(keys))
        }
        _ => unreachable!(),
    }
}
