use crate::{ParsedInputMap, ParsedInputValue, QueryGraphBuilderError, QueryGraphBuilderResult};
use query_structure::{prelude::ParentContainer, *};
use schema::constants::{aggregations, filters, json_null};
use std::convert::TryInto;

pub struct ScalarFilterParser<'a> {
    /// The field on which the filters are applied.
    field: &'a ScalarFieldRef,
    /// Whether it should extract inverted filters.
    reverse: bool,
    /// Whether the parser is going to parse a filter map that relates to a _count filter.
    is_count_filter: bool,
}

impl<'a> ScalarFilterParser<'a> {
    pub fn new(field: &'a ScalarFieldRef, reverse: bool) -> Self {
        Self {
            field,
            reverse,
            is_count_filter: false,
        }
    }

    fn field(&self) -> &ScalarFieldRef {
        self.field
    }

    fn reverse(&self) -> bool {
        self.reverse
    }

    fn is_count_filter(&self) -> bool {
        self.is_count_filter
    }

    /// Whether the parser is going to parse a filter map that relates to a _count filter
    /// Used to enable referencing fields of TypeIdentifier::Int although the field on which the
    /// filter is applied is of a different TypeIdentifier
    pub fn set_is_count_filter(mut self, is_count_filter: bool) -> Self {
        self.is_count_filter = is_count_filter;
        self
    }

    pub fn parse(&self, mut filter_map: ParsedInputMap<'_>) -> QueryGraphBuilderResult<Vec<Filter>> {
        let json_path: Option<JsonFilterPath> = match filter_map.swap_remove(filters::PATH) {
            Some(v) => Some(parse_json_path(v)?),
            _ => None,
        };

        let filters: Vec<Filter> = filter_map
            .into_iter()
            .map(|(name, value)| match self.field().type_identifier() {
                TypeIdentifier::Json => self.parse_json(&name, value, json_path.clone()),
                _ => self.parse_scalar(&name, value),
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

    fn parse_scalar(&self, filter_name: &str, input: ParsedInputValue<'_>) -> QueryGraphBuilderResult<Vec<Filter>> {
        let field = self.field();

        match filter_name {
            filters::NOT_LOWERCASE => {
                match input {
                    // Support for syntax `{ scalarField: { not: null } }` and `{ scalarField: { not: <value> } }`
                    ParsedInputValue::Single(value) => Ok(vec![field.not_equals(value)]),
                    _ => {
                        let inner_object: ParsedInputMap<'_> = input.try_into()?;

                        ScalarFilterParser::new(self.field, !self.reverse()).parse(inner_object)
                    }
                }
            }

            filters::IN => {
                let value = self.as_condition_value(input, true)?;

                let filter = match value {
                    ConditionValue::Value(value) => match value {
                        PrismaValue::Null if self.reverse() => field.not_equals(value),
                        PrismaValue::List(values) if self.reverse() => field.not_in(values),

                        PrismaValue::Null => field.equals(value),
                        PrismaValue::List(values) => field.is_in(values),

                        _ => unreachable!(), // Validation guarantees this.
                    },
                    ConditionValue::FieldRef(field_ref) if self.reverse() => field.not_in(field_ref),
                    ConditionValue::FieldRef(field_ref) => field.is_in(field_ref),
                };

                Ok(vec![filter])
            }

            // Legacy operation
            filters::NOT_IN => {
                let value = self.as_condition_value(input, true)?;

                let filter = match value {
                    ConditionValue::Value(value) => match value {
                        PrismaValue::Null if self.reverse() => field.equals(value), // not not in null => in null
                        PrismaValue::List(values) if self.reverse() => field.is_in(values), // not not in values => in values

                        PrismaValue::Null => field.not_equals(value),
                        PrismaValue::List(values) => field.not_in(values),

                        _ => unreachable!(), // Validation guarantees this.
                    },
                    ConditionValue::FieldRef(field_ref) if self.reverse() => field.is_in(field_ref),
                    ConditionValue::FieldRef(field_ref) => field.not_in(field_ref),
                };

                Ok(vec![filter])
            }

            filters::EQUALS if self.reverse() => Ok(vec![field.not_equals(self.as_condition_value(input, false)?)]),
            filters::CONTAINS if self.reverse() => Ok(vec![field.not_contains(self.as_condition_value(input, false)?)]),
            filters::STARTS_WITH if self.reverse() => {
                Ok(vec![field.not_starts_with(self.as_condition_value(input, false)?)])
            }
            filters::ENDS_WITH if self.reverse() => {
                Ok(vec![field.not_ends_with(self.as_condition_value(input, false)?)])
            }

            filters::EQUALS => Ok(vec![field.equals(self.as_condition_value(input, false)?)]),
            filters::CONTAINS => Ok(vec![field.contains(self.as_condition_value(input, false)?)]),
            filters::STARTS_WITH => Ok(vec![field.starts_with(self.as_condition_value(input, false)?)]),
            filters::ENDS_WITH => Ok(vec![field.ends_with(self.as_condition_value(input, false)?)]),

            filters::LOWER_THAN if self.reverse() => Ok(vec![
                field.greater_than_or_equals(self.as_condition_value(input, false)?)
            ]),
            filters::GREATER_THAN if self.reverse() => {
                Ok(vec![field.less_than_or_equals(self.as_condition_value(input, false)?)])
            }
            filters::LOWER_THAN_OR_EQUAL if self.reverse() => {
                Ok(vec![field.greater_than(self.as_condition_value(input, false)?)])
            }
            filters::GREATER_THAN_OR_EQUAL if self.reverse() => {
                Ok(vec![field.less_than(self.as_condition_value(input, false)?)])
            }

            filters::LOWER_THAN => Ok(vec![field.less_than(self.as_condition_value(input, false)?)]),
            filters::GREATER_THAN => Ok(vec![field.greater_than(self.as_condition_value(input, false)?)]),
            filters::LOWER_THAN_OR_EQUAL => Ok(vec![field.less_than_or_equals(self.as_condition_value(input, false)?)]),
            filters::GREATER_THAN_OR_EQUAL => Ok(vec![
                field.greater_than_or_equals(self.as_condition_value(input, false)?)
            ]),

            filters::SEARCH if self.reverse() => Ok(vec![field.not_search(self.as_condition_value(input, false)?)]),
            filters::SEARCH => Ok(vec![field.search(self.as_condition_value(input, false)?)]),

            filters::IS_SET if self.reverse() => {
                let is_set: bool = input.try_into()?;

                Ok(vec![field.is_set(!is_set)])
            }
            filters::IS_SET => Ok(vec![field.is_set(input.try_into()?)]),

            // List-specific filters
            filters::HAS => Ok(vec![field.contains_element(self.as_condition_value(input, false)?)]),
            filters::HAS_EVERY => Ok(vec![field.contains_every_element(self.as_condition_list_value(input)?)]),
            filters::HAS_SOME => Ok(vec![field.contains_some_element(self.as_condition_list_value(input)?)]),
            filters::IS_EMPTY => Ok(vec![field.is_empty_list(input.try_into()?)]),

            // Geometry-specific filters
            filters::GEO_WITHIN => {
                if self.reverse() {
                    Ok(vec![field.geometry_not_within(self.as_condition_value(input, false)?)])
                } else {
                    Ok(vec![field.geometry_within(self.as_condition_value(input, false)?)])
                }
            }
            filters::GEO_INTERSECTS => {
                if self.reverse() {
                    Ok(vec![
                        field.geometry_not_intersects(self.as_condition_value(input, false)?)
                    ])
                } else {
                    Ok(vec![field.geometry_intersects(self.as_condition_value(input, false)?)])
                }
            }

            // Aggregation filters
            aggregations::UNDERSCORE_COUNT => self.aggregation_filter(input, Filter::count, true),
            aggregations::UNDERSCORE_AVG => self.aggregation_filter(input, Filter::average, false),
            aggregations::UNDERSCORE_SUM => self.aggregation_filter(input, Filter::sum, false),
            aggregations::UNDERSCORE_MIN => self.aggregation_filter(input, Filter::min, false),
            aggregations::UNDERSCORE_MAX => self.aggregation_filter(input, Filter::max, false),

            _ => Err(QueryGraphBuilderError::InputError(format!(
                "{filter_name} is not a valid scalar filter operation"
            ))),
        }
    }

    fn parse_json(
        &self,
        filter_name: &str,
        input: ParsedInputValue<'_>,
        json_path: Option<JsonFilterPath>,
    ) -> QueryGraphBuilderResult<Vec<Filter>> {
        let field = self.field();

        match filter_name {
            filters::NOT_LOWERCASE => {
                match input {
                    // Support for syntax `{ scalarField: { not: <value> } }` and `{ scalarField: { not: <value> } }`
                    ParsedInputValue::Single(value) => {
                        let filter =
                            json_null_enum_filter(value, json_path, |val, path| field.json_not_equals(val, path), true);

                        Ok(vec![filter])
                    }
                    ParsedInputValue::Map(ref map) if matches!(map.tag, Some(schema::ObjectTag::FieldRefType(_))) => {
                        let filter = field.json_not_equals(self.as_condition_value(input, false)?, json_path);

                        Ok(vec![filter])
                    }
                    _ => {
                        let inner_object: ParsedInputMap<'_> = input.try_into()?;

                        ScalarFilterParser::new(self.field(), !self.reverse()).parse(inner_object)
                    }
                }
            }

            filters::EQUALS if self.reverse() => {
                let filter = json_null_enum_filter(
                    self.as_condition_value(input, false)?,
                    json_path,
                    |val, path| field.json_not_equals(val, path),
                    true,
                );

                Ok(vec![filter])
            }

            filters::EQUALS => {
                let filter = json_null_enum_filter(
                    self.as_condition_value(input, false)?,
                    json_path,
                    |val, path| field.json_equals(val, path),
                    false,
                );

                Ok(vec![filter])
            }

            filters::LOWER_THAN if self.reverse() => {
                Ok(vec![field.json_greater_than_or_equals(
                    self.as_condition_value(input, false)?,
                    json_path,
                )])
            }

            filters::GREATER_THAN if self.reverse() => {
                Ok(vec![field.json_less_than_or_equals(
                    self.as_condition_value(input, false)?,
                    json_path,
                )])
            }

            filters::LOWER_THAN_OR_EQUAL if self.reverse() => Ok(vec![
                field.json_greater_than(self.as_condition_value(input, false)?, json_path)
            ]),

            filters::GREATER_THAN_OR_EQUAL if self.reverse() => Ok(vec![
                field.json_less_than(self.as_condition_value(input, false)?, json_path)
            ]),
            filters::LOWER_THAN => Ok(vec![
                field.json_less_than(self.as_condition_value(input, false)?, json_path)
            ]),
            filters::GREATER_THAN => Ok(vec![
                field.json_greater_than(self.as_condition_value(input, false)?, json_path)
            ]),
            filters::LOWER_THAN_OR_EQUAL => {
                Ok(vec![field.json_less_than_or_equals(
                    self.as_condition_value(input, false)?,
                    json_path,
                )])
            }

            filters::GREATER_THAN_OR_EQUAL => {
                Ok(vec![field.json_greater_than_or_equals(
                    self.as_condition_value(input, false)?,
                    json_path,
                )])
            }

            // List-specific filters
            filters::HAS => Ok(vec![field.contains_element(self.as_condition_value(input, false)?)]),
            filters::HAS_EVERY => Ok(vec![field.contains_every_element(self.as_condition_list_value(input)?)]),
            filters::HAS_SOME => Ok(vec![field.contains_some_element(self.as_condition_list_value(input)?)]),
            filters::IS_EMPTY => Ok(vec![field.is_empty_list(input.try_into()?)]),

            // Json-specific filters
            filters::ARRAY_CONTAINS if self.reverse() => {
                let filter = json_null_enum_filter(
                    coerce_json_null(self.as_condition_value(input, false)?),
                    json_path,
                    |val, path| field.json_not_contains(val, path, JsonTargetType::Array),
                    true,
                );

                Ok(vec![filter])
            }

            filters::ARRAY_STARTS_WITH if self.reverse() => {
                let filter = json_null_enum_filter(
                    coerce_json_null(self.as_condition_value(input, false)?),
                    json_path,
                    |val, path| field.json_not_starts_with(val, path, JsonTargetType::Array),
                    true,
                );

                Ok(vec![filter])
            }

            filters::ARRAY_ENDS_WITH if self.reverse() => {
                let filter = json_null_enum_filter(
                    coerce_json_null(self.as_condition_value(input, false)?),
                    json_path,
                    |val, path| field.json_not_ends_with(val, path, JsonTargetType::Array),
                    true,
                );

                Ok(vec![filter])
            }

            filters::STRING_CONTAINS if self.reverse() => Ok(vec![field.json_not_contains(
                self.internal_as_condition_value(input, false, &TypeIdentifier::String)?,
                json_path,
                JsonTargetType::String,
            )]),

            filters::STRING_STARTS_WITH if self.reverse() => Ok(vec![field.json_not_starts_with(
                self.internal_as_condition_value(input, false, &TypeIdentifier::String)?,
                json_path,
                JsonTargetType::String,
            )]),

            filters::STRING_ENDS_WITH if self.reverse() => Ok(vec![field.json_not_ends_with(
                self.internal_as_condition_value(input, false, &TypeIdentifier::String)?,
                json_path,
                JsonTargetType::String,
            )]),

            filters::ARRAY_CONTAINS => {
                let filter = json_null_enum_filter(
                    coerce_json_null(self.as_condition_value(input, false)?),
                    json_path,
                    |val, path| field.json_contains(val, path, JsonTargetType::Array),
                    true,
                );

                Ok(vec![filter])
            }

            filters::ARRAY_STARTS_WITH => {
                let filter = json_null_enum_filter(
                    coerce_json_null(self.as_condition_value(input, false)?),
                    json_path,
                    |val, path| field.json_starts_with(val, path, JsonTargetType::Array),
                    true,
                );

                Ok(vec![filter])
            }

            filters::ARRAY_ENDS_WITH => {
                let filter = json_null_enum_filter(
                    self.as_condition_value(input, false)?,
                    json_path,
                    |val, path| field.json_ends_with(val, path, JsonTargetType::Array),
                    true,
                );

                Ok(vec![filter])
            }

            filters::STRING_CONTAINS => Ok(vec![field.json_contains(
                self.internal_as_condition_value(input, false, &TypeIdentifier::String)?,
                json_path,
                JsonTargetType::String,
            )]),

            filters::STRING_STARTS_WITH => Ok(vec![field.json_starts_with(
                self.internal_as_condition_value(input, false, &TypeIdentifier::String)?,
                json_path,
                JsonTargetType::String,
            )]),

            filters::STRING_ENDS_WITH => Ok(vec![field.json_ends_with(
                self.internal_as_condition_value(input, false, &TypeIdentifier::String)?,
                json_path,
                JsonTargetType::String,
            )]),

            _ => Err(QueryGraphBuilderError::InputError(format!(
                "{filter_name} is not a valid scalar filter operation"
            ))),
        }
    }

    fn as_condition_value(
        &self,
        input: ParsedInputValue<'_>,
        expect_list_ref: bool,
    ) -> QueryGraphBuilderResult<ConditionValue> {
        // If we're parsing a count filter, force the referenced field to be of TypeIdentifier::Int
        let expected_type = if self.is_count_filter() {
            TypeIdentifier::Int
        } else {
            self.field().type_identifier()
        };

        self.internal_as_condition_value(input, expect_list_ref, &expected_type)
    }

    fn internal_as_condition_value(
        &self,
        input: ParsedInputValue<'_>,
        expect_list_ref: bool,
        expected_type: &TypeIdentifier,
    ) -> QueryGraphBuilderResult<ConditionValue> {
        let field = self.field();

        match input {
            ParsedInputValue::Map(mut map) => {
                let field_ref_name = map.swap_remove(filters::UNDERSCORE_REF).unwrap();
                let field_ref_name = PrismaValue::try_from(field_ref_name)?.into_string().unwrap();
                let field_ref = field.container().find_field(&field_ref_name);

                let container_ref_name = map.swap_remove(filters::UNDERSCORE_CONTAINER).unwrap();
                let container_ref_name = PrismaValue::try_from(container_ref_name)?.into_string().unwrap();

                if container_ref_name != field.container().name() {
                    let expected_container_type = if field.container().is_model() {
                        "model"
                    } else {
                        "composite type"
                    };

                    let container_ref = field
                        .dm
                        .models()
                        .map(ParentContainer::from)
                        .chain(field.dm.composite_types().map(ParentContainer::from))
                        .find(|container| container.name() == container_ref_name)
                        .ok_or_else(|| {
                            QueryGraphBuilderError::InputError(format!(
                                "Model or composite type {} used for field ref {} does not exist.",
                                container_ref_name, field_ref_name
                            ))
                        })?;

                    let found_container_type = if container_ref.is_model() {
                        "model"
                    } else {
                        "composite type"
                    };

                    return Err(QueryGraphBuilderError::InputError(format!(
                        "Expected a referenced scalar field of {} {}, but found a field of {} {}.",
                        expected_container_type,
                        field.container().name(),
                        found_container_type,
                        container_ref_name
                    )));
                }

                match field_ref {
                    Some(Field::Scalar(field_ref))
                        if field_ref.is_list() == expect_list_ref && field_ref.type_identifier() == *expected_type =>
                    {
                        Ok(ConditionValue::reference(field_ref))
                    }
                    Some(Field::Scalar(field_ref)) => Err(QueryGraphBuilderError::InputError(format!(
                        "Expected a referenced scalar field of type {:?}{} but found {} of type {:?}{}.",
                        expected_type,
                        if field.is_list() { "[]" } else { "" },
                        field_ref,
                        field_ref.type_identifier(),
                        if field_ref.is_list() { "[]" } else { "" },
                    ))),
                    Some(Field::Relation(field_ref)) => Err(QueryGraphBuilderError::InputError(format!(
                        "Expected a referenced scalar field {field_ref} but found a relation field."
                    ))),
                    Some(Field::Composite(field_ref)) => Err(QueryGraphBuilderError::InputError(format!(
                        "Expected a referenced scalar field {field_ref} but found a composite field."
                    ))),
                    None => Err(QueryGraphBuilderError::InputError(format!(
                        "The referenced scalar field {}.{} does not exist.",
                        field.container().name(),
                        &field_ref_name
                    ))),
                }
            }
            _ => Ok(ConditionValue::value(input.try_into()?)),
        }
    }

    fn as_condition_list_value(&self, input: ParsedInputValue<'_>) -> QueryGraphBuilderResult<ConditionListValue> {
        let field = self.field();

        match input {
            ParsedInputValue::Map(mut map) => {
                let field_ref_name = map.swap_remove(filters::UNDERSCORE_REF).unwrap();
                let field_ref_name = PrismaValue::try_from(field_ref_name)?.into_string().unwrap();
                let field_ref = field.container().find_field(&field_ref_name);

                match field_ref {
                    Some(Field::Scalar(field_ref))
                        if field_ref.is_list() && field_ref.type_identifier() == field.type_identifier() =>
                    {
                        Ok(ConditionListValue::reference(field_ref))
                    }
                    Some(Field::Scalar(field_ref)) => Err(QueryGraphBuilderError::InputError(format!(
                        "Expected a referenced scalar field of type {:?}{} but found {} of type {:?}{}.",
                        field.type_identifier(),
                        if field.is_list() { "[]" } else { "" },
                        field_ref,
                        field_ref.type_identifier(),
                        if field_ref.is_list() { "[]" } else { "" },
                    ))),
                    Some(Field::Relation(rf)) => Err(QueryGraphBuilderError::InputError(format!(
                        "Expected a referenced scalar list field {rf} but found a relation field."
                    ))),
                    Some(Field::Composite(cf)) => Err(QueryGraphBuilderError::InputError(format!(
                        "Expected a referenced scalar list field {cf} but found a composite field."
                    ))),
                    _ => Err(QueryGraphBuilderError::InputError(format!(
                        "The referenced scalar list field {}.{} does not exist.",
                        field.container().name(),
                        &field_ref_name
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
        &self,
        input: ParsedInputValue<'_>,
        func: F,
        is_count_filter: bool,
    ) -> QueryGraphBuilderResult<Vec<Filter>>
    where
        F: Fn(Filter) -> Filter,
    {
        let inner_object: ParsedInputMap<'_> = input.try_into()?;
        let filters: Vec<Filter> = ScalarFilterParser::new(self.field, self.reverse())
            .set_is_count_filter(is_count_filter)
            .parse(inner_object)?;

        Ok(filters.into_iter().map(func).collect())
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
                    filter_fn(PrismaValue::Json("null".to_owned()).into(), json_path.clone()),
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
        ConditionValue::FieldRef(field_ref) => filter_fn(field_ref.into(), json_path),
    };

    filter
}

fn parse_json_path(input: ParsedInputValue<'_>) -> QueryGraphBuilderResult<JsonFilterPath> {
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
