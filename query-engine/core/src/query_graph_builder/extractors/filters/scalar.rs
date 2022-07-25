use crate::{ParsedInputMap, ParsedInputValue, QueryGraphBuilderError, QueryGraphBuilderResult};
use connector::{
    ConditionListValue, ConditionValue, Filter, JsonCompare, JsonFilterPath, JsonTargetType, ScalarCompare,
    ScalarListCompare,
};
use prisma_models::{Field, PrismaValue, ScalarFieldRef, TypeIdentifier};
use schema_builder::constants::{aggregations, filters, json_null};
use std::convert::TryInto;

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
            TypeIdentifier::Json => parse_internal_json(&k, v, field, json_path.clone(), reverse),
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
    json_path: Option<JsonFilterPath>,
    reverse: bool,
) -> QueryGraphBuilderResult<Vec<Filter>> {
    match filter_key {
        filters::NOT_LOWERCASE => {
            match input {
                // Support for syntax `{ scalarField: { not: <value> } }` and `{ scalarField: { not: <value> } }`
                ParsedInputValue::Single(value) => {
                    let filter =
                        json_null_enum_filter(value, json_path, |val, path| field.json_not_equals(val, path), true);

                    Ok(vec![filter])
                }
                _ => {
                    let inner_object: ParsedInputMap = input.try_into()?;

                    parse(inner_object, field, !reverse)
                }
            }
        }

        filters::EQUALS if reverse => {
            let filter = json_null_enum_filter(
                as_condition_value(input, field, false)?,
                json_path,
                |val, path| field.json_not_equals(val, path),
                true,
            );

            Ok(vec![filter])
        }

        filters::EQUALS => {
            let filter = json_null_enum_filter(
                as_condition_value(input, field, false)?,
                json_path,
                |val, path| field.json_equals(val, path),
                false,
            );

            Ok(vec![filter])
        }

        filters::LOWER_THAN if reverse => {
            Ok(vec![field.json_greater_than_or_equals(
                as_condition_value(input, field, false)?,
                json_path,
            )])
        }

        filters::GREATER_THAN if reverse => {
            Ok(vec![field.json_less_than_or_equals(
                as_condition_value(input, field, false)?,
                json_path,
            )])
        }

        filters::LOWER_THAN_OR_EQUAL if reverse => {
            Ok(vec![field.json_greater_than(
                as_condition_value(input, field, false)?,
                json_path,
            )])
        }

        filters::GREATER_THAN_OR_EQUAL if reverse => Ok(vec![
            field.json_less_than(as_condition_value(input, field, false)?, json_path)
        ]),
        filters::LOWER_THAN => Ok(vec![
            field.json_less_than(as_condition_value(input, field, false)?, json_path)
        ]),
        filters::GREATER_THAN => {
            Ok(vec![field.json_greater_than(
                as_condition_value(input, field, false)?,
                json_path,
            )])
        }
        filters::LOWER_THAN_OR_EQUAL => {
            Ok(vec![field.json_less_than_or_equals(
                as_condition_value(input, field, false)?,
                json_path,
            )])
        }

        filters::GREATER_THAN_OR_EQUAL => {
            Ok(vec![field.json_greater_than_or_equals(
                as_condition_value(input, field, false)?,
                json_path,
            )])
        }

        // List-specific filters
        filters::HAS => Ok(vec![field.contains_element(as_condition_value(input, field, false)?)]),
        filters::HAS_EVERY => Ok(vec![
            field.contains_every_element(as_condition_list_value(input, field)?)
        ]),
        filters::HAS_SOME => Ok(vec![field.contains_some_element(as_condition_list_value(input, field)?)]),
        filters::IS_EMPTY => Ok(vec![field.is_empty_list(input.try_into()?)]),

        // Json-specific filters
        filters::ARRAY_CONTAINS if reverse => {
            let filter = json_null_enum_filter(
                coerce_json_null(as_condition_value(input, field, false)?),
                json_path,
                |val, path| field.json_not_contains(val, path, JsonTargetType::Array),
                true,
            );

            Ok(vec![filter])
        }

        filters::ARRAY_STARTS_WITH if reverse => {
            let filter = json_null_enum_filter(
                coerce_json_null(as_condition_value(input, field, false)?),
                json_path,
                |val, path| field.json_not_starts_with(val, path, JsonTargetType::Array),
                true,
            );

            Ok(vec![filter])
        }

        filters::ARRAY_ENDS_WITH if reverse => {
            let filter = json_null_enum_filter(
                coerce_json_null(as_condition_value(input, field, false)?),
                json_path,
                |val, path| field.json_not_ends_with(val, path, JsonTargetType::Array),
                true,
            );

            Ok(vec![filter])
        }

        filters::STRING_CONTAINS if reverse => Ok(vec![field.json_not_contains(
            as_condition_value(input, field, false)?,
            json_path,
            JsonTargetType::String,
        )]),

        filters::STRING_STARTS_WITH if reverse => Ok(vec![field.json_not_starts_with(
            as_condition_value(input, field, false)?,
            json_path,
            JsonTargetType::String,
        )]),

        filters::STRING_ENDS_WITH if reverse => Ok(vec![field.json_not_ends_with(
            as_condition_value(input, field, false)?,
            json_path,
            JsonTargetType::String,
        )]),

        filters::ARRAY_CONTAINS => {
            let filter = json_null_enum_filter(
                coerce_json_null(as_condition_value(input, field, false)?),
                json_path,
                |val, path| field.json_contains(val, path, JsonTargetType::Array),
                true,
            );

            Ok(vec![filter])
        }

        filters::ARRAY_STARTS_WITH => {
            let filter = json_null_enum_filter(
                coerce_json_null(as_condition_value(input, field, false)?),
                json_path,
                |val, path| field.json_starts_with(val, path, JsonTargetType::Array),
                true,
            );

            Ok(vec![filter])
        }

        filters::ARRAY_ENDS_WITH => {
            let filter = json_null_enum_filter(
                as_condition_value(input, field, false)?,
                json_path,
                |val, path| field.json_ends_with(val, path, JsonTargetType::Array),
                true,
            );

            Ok(vec![filter])
        }

        filters::STRING_CONTAINS => Ok(vec![field.json_contains(
            as_condition_value(input, field, false)?,
            json_path,
            JsonTargetType::String,
        )]),

        filters::STRING_STARTS_WITH => Ok(vec![field.json_starts_with(
            as_condition_value(input, field, false)?,
            json_path,
            JsonTargetType::String,
        )]),

        filters::STRING_ENDS_WITH => Ok(vec![field.json_ends_with(
            as_condition_value(input, field, false)?,
            json_path,
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

fn json_null_enum_filter<F>(
    value: impl Into<ConditionValue>,
    json_path: Option<JsonFilterPath>,
    filter_fn: F,
    reverse: bool,
) -> Filter
where
    F: Fn(ConditionValue, Option<JsonFilterPath>) -> Filter,
{
    let filter = match value.into() {
        ConditionValue::Value(value) => match value {
            PrismaValue::Enum(e) => match e.as_str() {
                json_null::DB_NULL => filter_fn(PrismaValue::Null.into(), json_path),
                json_null::JSON_NULL => filter_fn(PrismaValue::Json("null".to_owned()).into(), json_path),

                json_null::ANY_NULL if reverse => Filter::And(vec![
                    filter_fn(PrismaValue::Json("null".to_owned()).into(), json_path.clone().into()),
                    filter_fn(PrismaValue::Null.into(), json_path),
                ]),

                json_null::ANY_NULL => Filter::Or(vec![
                    filter_fn(PrismaValue::Json("null".to_owned()).into(), json_path.clone()),
                    filter_fn(PrismaValue::Null.into(), json_path),
                ]),

                _ => unreachable!(), // Validation guarantees correct enum values.
            },
            val => filter_fn(val.into(), json_path),
        },
        ConditionValue::FieldRef(ref_field) => filter_fn(ref_field.into(), json_path),
    };

    filter
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
            let value = as_condition_value(input, field, true)?;

            let filter = match value {
                ConditionValue::Value(value) => match value {
                    PrismaValue::Null if reverse => field.not_equals(value),
                    PrismaValue::List(values) if reverse => field.not_in(values),

                    PrismaValue::Null => field.equals(value),
                    PrismaValue::List(values) => field.is_in(values),

                    _ => unreachable!(), // Validation guarantees this.
                },
                ConditionValue::FieldRef(ref_field) if reverse => field.not_in(ref_field),
                ConditionValue::FieldRef(ref_field) => field.is_in(ref_field),
            };

            Ok(vec![filter])
        }

        // Legacy operation
        filters::NOT_IN => {
            let value = as_condition_value(input, field, true)?;

            let filter = match value {
                ConditionValue::Value(value) => match value {
                    PrismaValue::Null if reverse => field.equals(value), // not not in null => in null
                    PrismaValue::List(values) if reverse => field.is_in(values), // not not in values => in values

                    PrismaValue::Null => field.not_equals(value),
                    PrismaValue::List(values) => field.not_in(values),

                    _ => unreachable!(), // Validation guarantees this.
                },
                ConditionValue::FieldRef(ref_field) if reverse => field.is_in(ref_field),
                ConditionValue::FieldRef(ref_field) => field.not_in(ref_field),
            };

            Ok(vec![filter])
        }

        filters::EQUALS if reverse => Ok(vec![field.not_equals(as_condition_value(input, field, false)?)]),
        filters::CONTAINS if reverse => Ok(vec![field.not_contains(as_condition_value(input, field, false)?)]),
        filters::STARTS_WITH if reverse => Ok(vec![field.not_starts_with(as_condition_value(input, field, false)?)]),
        filters::ENDS_WITH if reverse => Ok(vec![field.not_ends_with(as_condition_value(input, field, false)?)]),

        filters::EQUALS => Ok(vec![field.equals(as_condition_value(input, field, false)?)]),
        filters::CONTAINS => Ok(vec![field.contains(as_condition_value(input, field, false)?)]),
        filters::STARTS_WITH => Ok(vec![field.starts_with(as_condition_value(input, field, false)?)]),
        filters::ENDS_WITH => Ok(vec![field.ends_with(as_condition_value(input, field, false)?)]),

        filters::LOWER_THAN if reverse => Ok(vec![
            field.greater_than_or_equals(as_condition_value(input, field, false)?)
        ]),
        filters::GREATER_THAN if reverse => {
            Ok(vec![field.less_than_or_equals(as_condition_value(input, field, false)?)])
        }
        filters::LOWER_THAN_OR_EQUAL if reverse => {
            Ok(vec![field.greater_than(as_condition_value(input, field, false)?)])
        }
        filters::GREATER_THAN_OR_EQUAL if reverse => {
            Ok(vec![field.less_than(as_condition_value(input, field, false)?)])
        }

        filters::LOWER_THAN => Ok(vec![field.less_than(as_condition_value(input, field, false)?)]),
        filters::GREATER_THAN => Ok(vec![field.greater_than(as_condition_value(input, field, false)?)]),
        filters::LOWER_THAN_OR_EQUAL => Ok(vec![field.less_than_or_equals(as_condition_value(input, field, false)?)]),
        filters::GREATER_THAN_OR_EQUAL => Ok(vec![
            field.greater_than_or_equals(as_condition_value(input, field, false)?)
        ]),

        filters::SEARCH if reverse => Ok(vec![field.not_search(as_condition_value(input, field, false)?)]),
        filters::SEARCH => Ok(vec![field.search(as_condition_value(input, field, false)?)]),

        filters::IS_SET if reverse => {
            let is_set: bool = input.try_into()?;

            Ok(vec![field.is_set(!is_set)])
        }
        filters::IS_SET => Ok(vec![field.is_set(input.try_into()?)]),

        // List-specific filters
        filters::HAS => Ok(vec![field.contains_element(as_condition_value(input, field, false)?)]),
        filters::HAS_EVERY => Ok(vec![
            field.contains_every_element(as_condition_list_value(input, field)?)
        ]),
        filters::HAS_SOME => Ok(vec![field.contains_some_element(as_condition_list_value(input, field)?)]),
        filters::IS_EMPTY => Ok(vec![field.is_empty_list(input.try_into()?)]),

        // Aggregation filters
        aggregations::UNDERSCORE_COUNT => aggregation_filter(field, input, reverse, Filter::count),
        aggregations::UNDERSCORE_AVG => aggregation_filter(field, input, reverse, Filter::average),
        aggregations::UNDERSCORE_SUM => aggregation_filter(field, input, reverse, Filter::sum),
        aggregations::UNDERSCORE_MIN => aggregation_filter(field, input, reverse, Filter::min),
        aggregations::UNDERSCORE_MAX => aggregation_filter(field, input, reverse, Filter::max),

        _ => {
            return Err(QueryGraphBuilderError::InputError(format!(
                "{} is not a valid scalar filter operation",
                filter_key
            )))
        }
    }
}

fn as_condition_value(
    input: ParsedInputValue,
    field: &ScalarFieldRef,
    expect_scalar_list_ref: bool,
) -> QueryGraphBuilderResult<ConditionValue> {
    match input {
        ParsedInputValue::Map(mut map) => {
            let ref_field_name = map.remove(filters::REF).unwrap();
            let ref_field_name = PrismaValue::try_from(ref_field_name)?.into_string().unwrap();
            let ref_field = field.container().find_field(&ref_field_name);

            match ref_field {
                Some(Field::Scalar(sf))
                    if sf.is_list() == expect_scalar_list_ref && field.type_identifier == sf.type_identifier =>
                {
                    Ok(ConditionValue::reference(sf))
                }
                Some(Field::Scalar(sf)) => Err(QueryGraphBuilderError::InputError(format!(
                    "Expected a referenced scalar field of type {}{} but found {} of type {}{}.",
                    field.type_identifier,
                    if field.is_list() { "[]" } else { "" },
                    sf,
                    sf.type_identifier,
                    if sf.is_list() { "[]" } else { "" },
                ))),
                Some(Field::Relation(rf)) => Err(QueryGraphBuilderError::InputError(format!(
                    "Expected a referenced scalar field {} but found a relation field.",
                    rf
                ))),
                Some(Field::Composite(cf)) => Err(QueryGraphBuilderError::InputError(format!(
                    "Expected a referenced scalar field {} but found a composite field.",
                    cf
                ))),
                None => Err(QueryGraphBuilderError::InputError(format!(
                    "The referenced scalar field {}.{} does not exist.",
                    field.container.name(),
                    &ref_field_name
                ))),
            }
        }
        _ => Ok(ConditionValue::value(input.try_into()?)),
    }
}

fn as_condition_list_value(
    input: ParsedInputValue,
    field: &ScalarFieldRef,
) -> QueryGraphBuilderResult<ConditionListValue> {
    match input {
        ParsedInputValue::Map(mut map) => {
            let ref_field_name = map.remove(filters::REF).unwrap();
            let ref_field_name = PrismaValue::try_from(ref_field_name)?.into_string().unwrap();
            let ref_field = field.container.find_field(&ref_field_name);

            match ref_field {
                Some(Field::Scalar(sf)) if sf.is_list() && sf.type_identifier == field.type_identifier => {
                    Ok(ConditionListValue::reference(sf))
                }
                Some(Field::Scalar(sf)) => Err(QueryGraphBuilderError::InputError(format!(
                    "Expected a referenced scalar field of type {}{} but found {} of type {}{}.",
                    field.type_identifier,
                    if field.is_list() { "[]" } else { "" },
                    sf,
                    sf.type_identifier,
                    if sf.is_list() { "[]" } else { "" },
                ))),
                Some(Field::Relation(rf)) => Err(QueryGraphBuilderError::InputError(format!(
                    "Expected a referenced scalar list field {} but found a relation field.",
                    rf
                ))),
                Some(Field::Composite(cf)) => Err(QueryGraphBuilderError::InputError(format!(
                    "Expected a referenced scalar list field {} but found a composite field.",
                    cf
                ))),
                _ => Err(QueryGraphBuilderError::InputError(format!(
                    "The referenced scalar list field {}.{} does not exist.",
                    field.container.name(),
                    &ref_field_name
                ))),
            }
        }
        _ => {
            let vals: Vec<PrismaValue> = input.try_into()?;

            Ok(ConditionListValue::list(vals))
        }
    }
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

    Ok(filters.into_iter().map(func).collect())
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

fn coerce_json_null(value: ConditionValue) -> ConditionValue {
    match value {
        ConditionValue::Value(PrismaValue::Null) => ConditionValue::value(PrismaValue::Json("null".to_owned())),
        _ => value,
    }
}
