use super::*;
use crate::{executor::get_engine_protocol, schema::*};
use bigdecimal::{BigDecimal, ToPrimitive};
use chrono::prelude::*;
use core::fmt;
use indexmap::IndexSet;
use prisma_models::dml::{self, ValueGeneratorFn};
use prisma_value::PrismaValue;
use std::{borrow::Borrow, convert::TryFrom, str::FromStr, sync::Arc, vec};
use user_facing_errors::query_engine::validation::ValidationError;
use uuid::Uuid;

pub struct QueryDocumentParser {
    /// NOW() default value that's reused for all NOW() defaults on a single query
    default_now: PrismaValue,
}

impl QueryDocumentParser {
    pub fn new(default_now: PrismaValue) -> Self {
        QueryDocumentParser { default_now }
    }

    // Public entry point to parsing the query document (as denoted by `selections`) against the `schema_object`.
    pub fn parse(
        &self,
        selections: &[Selection],
        schema_object: &ObjectType,
        query_schema: &QuerySchema,
    ) -> QueryParserResult<ParsedObject> {
        self.parse_object(
            Path::default(),
            Path::default(),
            selections,
            schema_object,
            query_schema,
        )
    }

    /// Parses and validates a set of selections against a schema (output) object.
    /// On an output object, optional types designate whether or not an output field can be nulled.
    /// In contrast, nullable and optional types on an input object are separate concepts.
    /// The above is the reason we don't need to check nullability here, as it is done by the output
    /// validation in the serialization step.
    fn parse_object(
        &self,
        selection_path: Path,
        argument_path: Path,
        selections: &[Selection],
        schema_object: &ObjectType,
        query_schema: &QuerySchema,
    ) -> QueryParserResult<ParsedObject> {
        if selections.is_empty() {
            return Err(ValidationError::empty_selection(
                selection_path.segments(),
                conversions::schema_object_to_output_type_description(schema_object, query_schema),
            ));
        }

        selections
            .iter()
            .map(|selection| {
                let field_name = selection.name();
                match schema_object.find_field(field_name) {
                    Some(ref field) => self.parse_field(
                        selection_path.clone(),
                        argument_path.clone(),
                        selection,
                        field,
                        query_schema,
                    ),
                    None => Err(ValidationError::unknown_selection_field(
                        selection_path.add(field_name.to_owned()).segments(),
                        conversions::schema_object_to_output_type_description(schema_object, query_schema),
                    )),
                }
            })
            .collect::<QueryParserResult<Vec<_>>>()
            .map(|fields| ParsedObject { fields })
    }

    /// Parses and validates a selection against a schema (output) field.
    fn parse_field(
        &self,
        selection_path: Path,
        argument_path: Path,
        selection: &Selection,
        schema_field: &OutputFieldRef,
        query_schema: &QuerySchema,
    ) -> QueryParserResult<FieldPair> {
        let selection_path = selection_path.add(schema_field.name.clone());

        // Parse and validate all provided arguments for the field
        self.parse_arguments(
            selection_path.clone(),
            argument_path.clone(),
            schema_field,
            selection.arguments(),
            query_schema,
        )
        .and_then(|arguments| {
            if !selection.nested_selections().is_empty() && schema_field.field_type.is_scalar() {
                Err(ValidationError::selection_set_on_scalar(
                    selection.name().to_string(),
                    selection_path.segments(),
                ))
            } else {
                // If the output type of the field is an object type of any form, validate the sub selection as well.
                let nested_fields = schema_field.field_type.as_object_type(&query_schema.db).map(|obj| {
                    self.parse_object(
                        selection_path.clone(),
                        argument_path.clone(),
                        selection.nested_selections(),
                        obj,
                        query_schema,
                    )
                });

                let nested_fields = match nested_fields {
                    Some(sub) => Some(sub?),
                    None => None,
                };

                let schema_field = Arc::clone(schema_field);
                let parsed_field = ParsedField {
                    name: selection.name().to_string(),
                    alias: selection.alias().clone(),
                    arguments,
                    nested_fields,
                };

                Ok(FieldPair {
                    parsed_field,
                    schema_field,
                })
            }
        })
    }

    /// Parses and validates selection arguments against a schema defined field.
    fn parse_arguments(
        &self,
        selection_path: Path,
        argument_path: Path,
        schema_field: &OutputFieldRef,
        given_arguments: &[(String, ArgumentValue)],
        query_schema: &QuerySchema,
    ) -> QueryParserResult<Vec<ParsedArgument>> {
        let valid_argument_names: IndexSet<&str> = schema_field.arguments.iter().map(|arg| arg.name.as_str()).collect();
        let given_argument_names: IndexSet<&str> = given_arguments.iter().map(|arg| arg.0.as_str()).collect();
        let invalid_argument_names = given_argument_names.difference(&valid_argument_names);
        invalid_argument_names
            .map(|extra_arg| {
                let argument_path = argument_path.add(extra_arg.to_string());
                Err(ValidationError::unknown_argument(
                    selection_path.segments(),
                    argument_path.segments(),
                    conversions::schema_arguments_to_argument_description_vec(&schema_field.arguments, query_schema),
                ))
            })
            .collect::<QueryParserResult<Vec<()>>>()?;

        // Check remaining arguments
        schema_field
            .arguments
            .iter()
            .filter_map(|input_field_ref| {
                // Match schema argument field to an argument field in the incoming document.
                let selection_arg: Option<(String, ArgumentValue)> = given_arguments
                    .iter()
                    .find(|given_argument| given_argument.0 == input_field_ref.name)
                    .cloned();

                let argument_path = argument_path.add(input_field_ref.name.clone());

                // If optional and not present ignore the field.
                // If present, parse normally.
                // If not present but required, throw a validation error.
                match selection_arg {
                    Some((_, value)) => Some(
                        self.parse_input_value(
                            selection_path.clone(),
                            argument_path,
                            value,
                            input_field_ref.field_types(query_schema),
                            query_schema,
                        )
                        .map(|value| ParsedArgument {
                            name: input_field_ref.name.clone(),
                            value,
                        }),
                    ),
                    None if !input_field_ref.is_required => None,
                    _ => Some(Err(ValidationError::required_argument_missing(
                        selection_path.segments(),
                        argument_path.segments(),
                        &conversions::input_types_to_input_type_descriptions(
                            input_field_ref.field_types(query_schema),
                            query_schema,
                        ),
                    ))),
                }
            })
            .collect::<Vec<QueryParserResult<ParsedArgument>>>()
            .into_iter()
            .collect()
    }

    /// Parses and validates an ArgumentValue against possible input types.
    /// Matching is done in order of definition on the input type. First matching type wins.
    fn parse_input_value(
        &self,
        selection_path: Path,
        argument_path: Path,
        value: ArgumentValue,
        possible_input_types: &[InputType],
        query_schema: &QuerySchema,
    ) -> QueryParserResult<ParsedInputValue> {
        let mut parse_results = vec![];

        for input_type in possible_input_types {
            let result = match (value.clone(), input_type) {
                // With the JSON protocol, JSON values are sent as deserialized values.
                // This means JSON can match with pretty much anything. A string, an int, an object, an array.
                // This is an early catch-all.
                // We do not get into this catch-all _if_ the value is already Json, if it's a FieldRef or if it's an Enum.
                // We don't because they've already been desambiguified at the procotol adapter level.
                (value, InputType::Scalar(ScalarType::Json))
                    if value.can_be_parsed_as_json() && get_engine_protocol().is_json() =>
                {
                    Ok(ParsedInputValue::Single(self.to_json(
                        &selection_path,
                        &argument_path,
                        &value,
                    )?))
                }
                // With the JSON protocol, JSON values are sent as deserialized values.
                // This means that a JsonList([1, 2]) will be coerced as an `ArgumentValue::List([1, 2])`.
                // We need this early matcher to make sure we coerce this array back to JSON.
                (list @ ArgumentValue::List(_), InputType::Scalar(ScalarType::JsonList))
                    if get_engine_protocol().is_json() =>
                {
                    let json_val = serde_json::to_value(list.clone()).map_err(|err| {
                        ValidationError::invalid_argument_value(
                            selection_path.segments(),
                            argument_path.segments(),
                            format!("{list:?}"),
                            String::from("JSON array"),
                            Some(Box::new(err)),
                        )
                    })?;
                    let json_list = self.parse_json_list_from_value(&selection_path, &argument_path, json_val)?;

                    Ok(ParsedInputValue::Single(json_list))
                }
                (ArgumentValue::Scalar(pv), input_type) => match (pv, input_type) {
                    // Null handling
                    (PrismaValue::Null, InputType::Scalar(ScalarType::Null)) => {
                        Ok(ParsedInputValue::Single(PrismaValue::Null))
                    }
                    (PrismaValue::Null, _) => Err(ValidationError::required_argument_missing(
                        selection_path.segments(),
                        argument_path.segments(),
                        &conversions::input_types_to_input_type_descriptions(&[input_type.clone()], query_schema),
                    )),
                    // Scalar handling
                    (pv, InputType::Scalar(st)) => self
                        .parse_scalar(
                            &selection_path,
                            &argument_path,
                            pv,
                            st,
                            &value,
                            input_type,
                            query_schema,
                        )
                        .map(ParsedInputValue::Single),

                    // Enum handling
                    (pv @ PrismaValue::Enum(_), InputType::Enum(et)) => {
                        self.parse_enum(&selection_path, &argument_path, pv, &query_schema.db[*et])
                    }
                    (pv @ PrismaValue::String(_), InputType::Enum(et)) => {
                        self.parse_enum(&selection_path, &argument_path, pv, &query_schema.db[*et])
                    }
                    (pv @ PrismaValue::Boolean(_), InputType::Enum(et)) => {
                        self.parse_enum(&selection_path, &argument_path, pv, &query_schema.db[*et])
                    }
                    // Invalid combinations
                    _ => Err(ValidationError::invalid_argument_type(
                        selection_path.segments(),
                        argument_path.segments(),
                        conversions::input_type_to_argument_description(
                            argument_path.last().unwrap_or_default().to_string(),
                            input_type,
                            query_schema,
                        ),
                        conversions::argument_value_to_type_name(&value),
                    )),
                },

                // List handling.
                (ArgumentValue::List(values), InputType::List(l)) => self
                    .parse_list(&selection_path, &argument_path, values.clone(), l, query_schema)
                    .map(ParsedInputValue::List),

                // Object handling
                (ArgumentValue::Object(o) | ArgumentValue::FieldRef(o), InputType::Object(obj)) => self
                    .parse_input_object(
                        selection_path.clone(),
                        argument_path.clone(),
                        o.clone(),
                        &query_schema.db[*obj],
                        query_schema,
                    )
                    .map(ParsedInputValue::Map),

                // Invalid combinations
                _ => Err(ValidationError::invalid_argument_type(
                    selection_path.segments(),
                    argument_path.segments(),
                    conversions::input_type_to_argument_description(
                        argument_path.last().unwrap_or_default().to_string(),
                        input_type,
                        query_schema,
                    ),
                    conversions::argument_value_to_type_name(&value),
                )),
            };

            parse_results.push(result);
        }

        let (successes, mut failures): (Vec<_>, Vec<_>) = parse_results.into_iter().partition(|result| result.is_ok());

        if successes.is_empty() {
            if failures.len() == 1 {
                failures.pop().unwrap()
            } else {
                Err(ValidationError::union(
                    failures
                        .into_iter()
                        .map(|err| match err {
                            Err(e) => e,
                            Ok(_) => unreachable!("Expecting to only have Result::Err in the `failures` vector."),
                        })
                        .collect(),
                ))
            }
        } else {
            successes.into_iter().next().unwrap()
        }
    }

    /// Attempts to parse given query value into a concrete PrismaValue based on given scalar type.
    #[allow(clippy::too_many_arguments)]
    fn parse_scalar(
        &self,
        selection_path: &Path,
        argument_path: &Path,
        value: PrismaValue,
        scalar_type: &ScalarType,
        argument_value: &ArgumentValue,
        input_type: &InputType,
        query_schema: &QuerySchema,
    ) -> QueryParserResult<PrismaValue> {
        match (value, scalar_type.clone()) {
            // Identity matchers
            (PrismaValue::String(s), ScalarType::String) => Ok(PrismaValue::String(s)),
            (PrismaValue::Boolean(b), ScalarType::Boolean) => Ok(PrismaValue::Boolean(b)),
            (PrismaValue::Json(json), ScalarType::Json) => Ok(PrismaValue::Json(json)),
            (PrismaValue::Xml(xml), ScalarType::Xml) => Ok(PrismaValue::Xml(xml)),
            (PrismaValue::Uuid(uuid), ScalarType::UUID) => Ok(PrismaValue::Uuid(uuid)),
            (PrismaValue::Bytes(bytes), ScalarType::Bytes) => Ok(PrismaValue::Bytes(bytes)),
            (PrismaValue::BigInt(b_int), ScalarType::BigInt) => Ok(PrismaValue::BigInt(b_int)),
            (PrismaValue::DateTime(s), ScalarType::DateTime) => Ok(PrismaValue::DateTime(s)),
            (PrismaValue::Null, ScalarType::Null) => Ok(PrismaValue::Null),

            // String coercion matchers
            (PrismaValue::String(s), ScalarType::Xml) => Ok(PrismaValue::Xml(s)),
            (PrismaValue::String(s), ScalarType::JsonList) => {
                self.parse_json_list_from_str(selection_path, argument_path, &s)
            }
            (PrismaValue::String(s), ScalarType::Bytes) => self.parse_bytes(selection_path, argument_path, s),
            (PrismaValue::String(s), ScalarType::Decimal) => self.parse_decimal(selection_path, argument_path, s),
            (PrismaValue::String(s), ScalarType::BigInt) => self.parse_bigint(selection_path, argument_path, s),
            (PrismaValue::String(s), ScalarType::UUID) => self
                .parse_uuid(selection_path, argument_path, s.as_str())
                .map(PrismaValue::Uuid),
            (PrismaValue::String(s), ScalarType::Json) => Ok(PrismaValue::Json(
                self.parse_json(selection_path, argument_path, &s).map(|_| s)?,
            )),
            (PrismaValue::String(s), ScalarType::DateTime) => self
                .parse_datetime(selection_path, argument_path, s.as_str())
                .map(PrismaValue::DateTime),

            // Int coercion matchers
            (PrismaValue::Int(i), ScalarType::Int) => Ok(PrismaValue::Int(i)),
            (PrismaValue::Int(i), ScalarType::Float) => Ok(PrismaValue::Float(BigDecimal::from(i))),
            (PrismaValue::Int(i), ScalarType::Decimal) => Ok(PrismaValue::Float(BigDecimal::from(i))),
            (PrismaValue::Int(i), ScalarType::BigInt) => Ok(PrismaValue::BigInt(i)),

            // Float coercion matchers
            (PrismaValue::Float(f), ScalarType::Float) => Ok(PrismaValue::Float(f)),
            (PrismaValue::Float(f), ScalarType::Decimal) => Ok(PrismaValue::Float(f)),
            (PrismaValue::Float(f), ScalarType::Int) => match f.to_i64() {
                Some(converted) => Ok(PrismaValue::Int(converted)),
                None => Err(ValidationError::value_too_large(
                    selection_path.segments(),
                    argument_path.segments(),
                    f.to_string(),
                )),
            },

            // UUID coercion matchers
            (PrismaValue::Uuid(uuid), ScalarType::String) => Ok(PrismaValue::String(uuid.to_string())),

            // All other combinations are value type mismatches.
            (_, _) => Err(ValidationError::invalid_argument_type(
                selection_path.segments(),
                argument_path.segments(),
                conversions::input_type_to_argument_description(
                    argument_path.last().unwrap_or_default().to_string(),
                    input_type,
                    query_schema,
                ),
                conversions::argument_value_to_type_name(argument_value),
            )),
        }
    }

    fn parse_datetime(
        &self,
        selection_path: &Path,
        argument_path: &Path,
        s: &str,
    ) -> QueryParserResult<DateTime<FixedOffset>> {
        prisma_value::parse_datetime(s).map_err(|err| {
            ValidationError::invalid_argument_value(
                selection_path.segments(),
                argument_path.segments(),
                s.to_string(),
                String::from("ISO-8601 DateTime"),
                Some(Box::new(err)),
            )
        })
    }

    fn parse_bytes(&self, selection_path: &Path, argument_path: &Path, s: String) -> QueryParserResult<PrismaValue> {
        prisma_value::decode_bytes(&s).map(PrismaValue::Bytes).map_err(|err| {
            ValidationError::invalid_argument_value(
                selection_path.segments(),
                argument_path.segments(),
                s.to_string(),
                String::from("base64 String"),
                Some(Box::new(err)),
            )
        })
    }

    fn parse_decimal(
        &self,
        selection_path: &Path,
        argument_path: &Path,
        value: String,
    ) -> QueryParserResult<PrismaValue> {
        BigDecimal::from_str(&value).map(PrismaValue::Float).map_err(|err| {
            ValidationError::invalid_argument_value(
                selection_path.segments(),
                argument_path.segments(),
                value,
                String::from("decimal String"),
                Some(Box::new(err)),
            )
        })
    }

    fn parse_bigint(
        &self,
        selection_path: &Path,
        argument_path: &Path,
        value: String,
    ) -> QueryParserResult<PrismaValue> {
        value.parse::<i64>().map(PrismaValue::BigInt).map_err(|err| {
            ValidationError::invalid_argument_value(
                selection_path.segments(),
                argument_path.segments(),
                value,
                String::from("big integer String"),
                Some(Box::new(err)),
            )
        })
    }

    fn parse_json_list_from_str(
        &self,
        selection_path: &Path,
        argument_path: &Path,
        value: &str,
    ) -> QueryParserResult<PrismaValue> {
        let json = self.parse_json(selection_path, argument_path, value)?;
        self.parse_json_list_from_value(selection_path, argument_path, json)
    }

    fn parse_json_list_from_value(
        &self,
        selection_path: &Path,
        argument_path: &Path,
        json: serde_json::Value,
    ) -> QueryParserResult<PrismaValue> {
        let values = json.as_array().ok_or_else(|| {
            ValidationError::invalid_argument_value(
                selection_path.segments(),
                argument_path.segments(),
                json.to_string(),
                String::from("JSON array"),
                None,
            )
        })?;

        let mut prisma_values = Vec::with_capacity(values.len());

        for v in values.iter() {
            let pv = PrismaValue::try_from(v.clone()).map_err(|err| {
                ValidationError::invalid_argument_value(
                    selection_path.segments(),
                    argument_path.segments(),
                    json.to_string(),
                    String::from("Flat JSON array (no nesting)"),
                    Some(Box::new(err)),
                )
            })?;

            prisma_values.push(pv);
        }

        Ok(PrismaValue::List(prisma_values))
    }

    fn parse_json(&self, selection_path: &Path, argument_path: &Path, s: &str) -> QueryParserResult<serde_json::Value> {
        serde_json::from_str(s).map_err(|err| {
            ValidationError::invalid_argument_value(
                selection_path.segments(),
                argument_path.segments(),
                s.to_string(),
                String::from("JSON String"),
                Some(Box::new(err)),
            )
        })
    }

    fn to_json(
        &self,
        selection_path: &Path,
        argument_path: &Path,
        value: &ArgumentValue,
    ) -> QueryParserResult<PrismaValue> {
        serde_json::to_string(&value)
            .map_err(|err| {
                ValidationError::invalid_argument_value(
                    selection_path.segments(),
                    argument_path.segments(),
                    format!("{value:?}"),
                    String::from("JSON String"),
                    Some(Box::new(err)),
                )
            })
            .map(PrismaValue::Json)
    }

    fn parse_uuid(&self, selection_path: &Path, argument_path: &Path, s: &str) -> QueryParserResult<Uuid> {
        Uuid::parse_str(s).map_err(|err| {
            ValidationError::invalid_argument_value(
                selection_path.segments(),
                argument_path.segments(),
                s.to_string(),
                String::from("UUID String"),
                Some(Box::new(err)),
            )
        })
    }

    fn parse_list(
        &self,
        selection_path: &Path,
        argument_path: &Path,
        values: Vec<ArgumentValue>,
        value_type: &InputType,
        query_schema: &QuerySchema,
    ) -> QueryParserResult<Vec<ParsedInputValue>> {
        values
            .into_iter()
            .map(|val| {
                self.parse_input_value(
                    selection_path.clone(),
                    argument_path.clone(),
                    val,
                    &[value_type.clone()],
                    query_schema,
                )
            })
            .collect::<QueryParserResult<Vec<ParsedInputValue>>>()
    }

    fn parse_enum(
        &self,
        selection_path: &Path,
        argument_path: &Path,
        val: PrismaValue,
        typ: &EnumType,
    ) -> QueryParserResult<ParsedInputValue> {
        let raw = match val {
            PrismaValue::Enum(s) => s,
            PrismaValue::String(s) => s,
            PrismaValue::Boolean(b) => if b { "true" } else { "false" }.to_owned(), // Case where a bool was misinterpreted as constant literal
            _ => {
                return Err(ValidationError::invalid_argument_value(
                    selection_path.segments(),
                    argument_path.segments(),
                    format!("{val:?}"),
                    typ.name(),
                    None,
                ));
            }
        };

        let err = |name: &str| {
            Err(ValidationError::invalid_argument_value(
                selection_path.segments(),
                argument_path.segments(),
                raw.clone(),
                name.to_string(),
                None,
            ))
        };

        match typ.borrow() {
            EnumType::Database(db) => match db.map_input_value(&raw) {
                Some(value) => Ok(ParsedInputValue::Single(value)),
                None => err(&db.identifier().name()),
            },
            EnumType::String(s) => match s.value_for(raw.as_str()) {
                Some(val) => Ok(ParsedInputValue::Single(PrismaValue::Enum(val.to_owned()))),
                None => err(&s.identifier().name()),
            },
            EnumType::FieldRef(f) => match f.value_for(raw.as_str()) {
                Some(value) => Ok(ParsedInputValue::ScalarField(value.clone())),
                None => err(&f.identifier().name()),
            },
        }
    }

    /// Parses and validates an input object recursively.
    fn parse_input_object(
        &self,
        selection_path: Path,
        argument_path: Path,
        object: ArgumentValueObject,
        schema_object: &InputObjectType,
        query_schema: &QuerySchema,
    ) -> QueryParserResult<ParsedInputMap> {
        let valid_field_names: IndexSet<&str> = schema_object
            .get_fields()
            .iter()
            .map(|field| field.name.as_str())
            .collect();
        let given_field_names: IndexSet<&str> = object.iter().map(|(k, _)| k.as_str()).collect();
        let missing_field_names = valid_field_names.difference(&given_field_names);

        // First, filter-in those fields that are not given but have a default value in the schema.
        // As in practise, it is like if they were given with said default value.
        let defaults = missing_field_names
            .filter_map(|unset_field_name| {
                let field = schema_object.find_field(*unset_field_name).unwrap();
                let argument_path = argument_path.add(field.name.clone());

                // If the input field has a default, add the default to the result.
                // If it's not optional and has no default, a required field has not been provided.
                match &field.default_value {
                    Some(default_value) => {
                        let default_pv = match &default_value {
                            dml::DefaultKind::Expression(ref expr)
                                if matches!(expr.generator(), ValueGeneratorFn::Now) =>
                            {
                                self.default_now.clone()
                            }
                            _ => default_value.get()?,
                        };

                        match self.parse_input_value(
                            selection_path.clone(),
                            argument_path,
                            default_pv.into(),
                            field.field_types(query_schema),
                            query_schema,
                        ) {
                            Ok(value) => Some(Ok((field.name.clone(), value))),
                            Err(err) => Some(Err(err)),
                        }
                    }
                    None => {
                        if field.is_required {
                            Some(Err(ValidationError::required_argument_missing(
                                selection_path.segments(),
                                argument_path.segments(),
                                &conversions::input_types_to_input_type_descriptions(
                                    field.field_types(query_schema),
                                    query_schema,
                                ),
                            )))
                        } else {
                            None
                        }
                    }
                }
            })
            .collect::<QueryParserResult<Vec<_>>>()?;

        // Checks all fields on the provided input object. This will catch extra
        // or unknown fields and parsing errors.
        let mut map = object
            .into_iter()
            .map(|(field_name, value)| {
                let field = schema_object.find_field(field_name.as_str()).ok_or_else(|| {
                    ValidationError::unknown_input_field(
                        selection_path.segments(),
                        argument_path.add(field_name.clone()).segments(),
                        conversions::schema_input_object_type_to_input_type_description(schema_object, query_schema),
                    )
                })?;

                let argument_path = argument_path.add(field.name.clone());
                let parsed = self.parse_input_value(
                    selection_path.clone(),
                    argument_path,
                    value,
                    field.field_types(query_schema),
                    query_schema,
                )?;

                Ok((field_name, parsed))
            })
            .collect::<QueryParserResult<ParsedInputMap>>()?;

        map.extend(defaults.into_iter());

        // Ensure the constraints are upheld. If any `fields` are specified, then the constraints should be upheld against those only.
        // If no `fields` are specified, then the constraints should be upheld against all fields of the object.
        let num_fields = schema_object
            .constraints
            .fields
            .as_ref()
            .cloned()
            .map(|fields| {
                fields.iter().fold(0, |mut acc, field| {
                    if map.contains_key(field) {
                        acc += 1;
                    }

                    acc
                })
            })
            .unwrap_or(map.len());

        let too_many = schema_object
            .constraints
            .max_num_fields
            .map(|max| num_fields > max)
            .unwrap_or(false);
        if too_many {
            return Err(ValidationError::too_many_fields_given(
                selection_path.segments(),
                argument_path.segments(),
                schema_object.constraints.min_num_fields,
                schema_object.constraints.max_num_fields,
                schema_object.constraints.fields.as_ref().cloned(),
                num_fields,
                &conversions::schema_input_object_type_to_input_type_description(schema_object, query_schema),
            ));
        }

        let some_missing = schema_object
            .constraints
            .min_num_fields
            .map(|min| num_fields < min)
            .unwrap_or(false);
        if some_missing {
            return Err(ValidationError::some_fields_missing(
                selection_path.segments(),
                argument_path.segments(),
                schema_object.constraints.min_num_fields,
                schema_object.constraints.max_num_fields,
                schema_object.constraints.fields.as_ref().cloned(),
                num_fields,
                &conversions::schema_input_object_type_to_input_type_description(schema_object, query_schema),
            ));
        }

        map.set_tag(schema_object.tag.clone());

        Ok(map)
    }
}

pub(crate) mod conversions {
    use crate::{
        schema::{InputType, OutputType},
        ArgumentValue,
    };
    use prisma_models::PrismaValue;
    use schema::QuerySchema;
    use user_facing_errors::query_engine::validation::{self, InputTypeDescription};

    /// converts an schema object to the narrower validation::OutputTypeDescription
    /// representation of an output field that is part of a validation error information.
    pub(crate) fn schema_object_to_output_type_description(
        o: &schema::ObjectType,
        query_schema: &QuerySchema,
    ) -> validation::OutputTypeDescription {
        let name = o.identifier.name();
        let fields: Vec<validation::OutputTypeDescriptionField> = o
            .get_fields()
            .iter()
            .map(|field| {
                let name = field.name.to_owned();
                let type_name = to_simplified_output_type_name(field.field_type.as_ref(), query_schema);
                let is_relation = field.maps_to_relation(query_schema);

                validation::OutputTypeDescriptionField::new(name, type_name, is_relation)
            })
            .collect();
        validation::OutputTypeDescription::new(name, fields)
    }

    pub(crate) fn input_types_to_input_type_descriptions(
        input_types: &[schema::InputType],
        query_schema: &QuerySchema,
    ) -> Vec<validation::InputTypeDescription> {
        input_types
            .iter()
            .map(|it| input_type_to_input_type_description(it, query_schema))
            .collect()
    }

    fn input_type_to_input_type_description(
        input_type: &InputType,
        query_schema: &QuerySchema,
    ) -> InputTypeDescription {
        match input_type {
            InputType::Scalar(s) => InputTypeDescription::Scalar { name: s.to_string() },
            InputType::Enum(e) => InputTypeDescription::Enum {
                name: query_schema.db[*e].name(),
            },
            InputType::List(l) => InputTypeDescription::List {
                element_type: Box::new(input_type_to_input_type_description(l.as_ref(), query_schema)),
            },
            InputType::Object(object_id) => {
                schema_input_object_type_to_input_type_description(&query_schema.db[*object_id], query_schema)
            }
        }
    }

    pub(crate) fn schema_input_object_type_to_input_type_description(
        i: &schema::InputObjectType,
        query_schema: &QuerySchema,
    ) -> validation::InputTypeDescription {
        let name = i.identifier.name();
        let fields: Vec<validation::InputTypeDescriptionField> = i
            .get_fields()
            .iter()
            .map(|field| {
                let name = field.name.clone();
                let type_names: Vec<String> = field
                    .field_types(query_schema)
                    .iter()
                    .map(|t| to_simplified_input_type_name(t, query_schema))
                    .collect();
                validation::InputTypeDescriptionField::new(name, type_names, field.is_required)
            })
            .collect();
        validation::InputTypeDescription::new_object(name, fields)
    }

    pub(crate) fn schema_arguments_to_argument_description_vec(
        arguments: &[schema::InputField],
        query_schema: &QuerySchema,
    ) -> Vec<validation::ArgumentDescription> {
        arguments
            .iter()
            .map(|input_field_ref| {
                validation::ArgumentDescription::new(
                    input_field_ref.name.to_string(),
                    input_field_ref
                        .field_types(query_schema)
                        .iter()
                        .map(|t| to_simplified_input_type_name(t, query_schema))
                        .collect(),
                )
            })
            .collect::<Vec<_>>()
    }

    pub(crate) fn input_type_to_argument_description(
        arg_name: String,
        input_type: &InputType,
        query_schema: &QuerySchema,
    ) -> validation::ArgumentDescription {
        validation::ArgumentDescription::new(arg_name, vec![to_simplified_input_type_name(input_type, query_schema)])
    }

    pub(crate) fn argument_value_to_type_name(value: &ArgumentValue) -> String {
        match value {
            ArgumentValue::Scalar(pv) => prisma_value_to_type_name(pv),
            ArgumentValue::Object(_) => "Object".to_string(),
            ArgumentValue::List(v) => {
                format!("({})", itertools::join(v.iter().map(argument_value_to_type_name), ", "))
            }
            ArgumentValue::FieldRef(_) => "FieldRef".to_string(),
        }
    }

    fn prisma_value_to_type_name(pv: &PrismaValue) -> String {
        match pv {
            PrismaValue::String(_) => "String".to_string(),
            PrismaValue::Boolean(_) => "Boolean".to_string(),
            PrismaValue::Enum(_) => "Enum".to_string(),
            PrismaValue::Int(_) => "Int".to_string(),
            PrismaValue::Uuid(_) => "UUID".to_string(),
            PrismaValue::List(v) => {
                format!("({})", itertools::join(v.iter().map(prisma_value_to_type_name), ", "))
            }
            PrismaValue::Json(_) => "JSON".to_string(),
            PrismaValue::Xml(_) => "XML".to_string(),
            PrismaValue::Object(_) => "Object".to_string(),
            PrismaValue::Null => "Null".to_string(),
            PrismaValue::DateTime(_) => "DateTime".to_string(),
            PrismaValue::Float(_) => "Float".to_string(),
            PrismaValue::BigInt(_) => "BigInt".to_string(),
            PrismaValue::Bytes(_) => "Bytes".to_string(),
        }
    }

    fn to_simplified_input_type_name(typ: &InputType, query_schema: &QuerySchema) -> String {
        match typ {
            InputType::Enum(e) => query_schema.db[*e].name(),
            InputType::List(o) => format!("{}[]", to_simplified_input_type_name(o.as_ref(), query_schema)),
            InputType::Object(object_id) => query_schema.db[*object_id].identifier.name(),
            InputType::Scalar(s) => s.to_string(),
        }
    }

    fn to_simplified_output_type_name(typ: &OutputType, query_schema: &QuerySchema) -> String {
        match typ {
            OutputType::Enum(e) => query_schema.db[*e].name(),
            OutputType::List(o) => format!("{}[]", to_simplified_output_type_name(o, query_schema)),
            OutputType::Object(o) => query_schema.db[*o].identifier.name(),
            OutputType::Scalar(s) => s.to_string(),
        }
    }
}
#[derive(Debug, Clone, Default)]
pub(crate) struct Path {
    segments: Vec<String>,
}

impl Path {
    pub(crate) fn add(&self, segment: String) -> Self {
        let mut path = self.clone();
        path.segments.push(segment);
        path
    }

    pub(crate) fn last(&self) -> Option<&str> {
        self.segments.last().map(|s| s.as_str())
    }

    pub(crate) fn segments(&self) -> Vec<String> {
        self.segments.clone()
    }
}

impl fmt::Display for Path {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.segments.join("."))
    }
}
