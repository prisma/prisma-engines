use super::*;
use crate::{executor::get_engine_protocol, schema::*};
use bigdecimal::{BigDecimal, ToPrimitive};
use chrono::prelude::*;
use indexmap::IndexSet;
use prisma_models::dml::{self, ValueGeneratorFn};
use prisma_value::PrismaValue;
use std::{borrow::Borrow, convert::TryFrom, str::FromStr, sync::Arc};
use user_facing_errors::query_engine::validation::{self, ValidationError};
use uuid::Uuid;

pub struct QueryDocumentParser {
    /// NOW() default value that's reused for all NOW() defaults on a single query
    default_now: PrismaValue,
}

impl QueryDocumentParser {
    pub fn new(default_now: PrismaValue) -> Self {
        QueryDocumentParser { default_now }
    }

    /// Parses and validates a set of selections against a schema (output) object.
    /// On an output object, optional types designate whether or not an output field can be nulled.
    /// In contrast, nullable and optional types on an input object are separate concepts.
    /// The above is the reason we don't need to check nullability here, as it is done by the output
    /// validation in the serialization step.
    pub fn parse_object(
        &self,
        parent_path: QueryPath,
        selection_path: SelectionPath,
        argument_path: ArgumentPath,
        selections: &[Selection],
        schema_object: &ObjectTypeStrongRef,
    ) -> QueryParserResult<ParsedObject> {
        let path = parent_path.add(schema_object.identifier.name().to_owned());

        if selections.is_empty() {
            return Err(ValidationError::empty_selection(
                selection_path.segments(),
                conversions::schema_object_to_output_type_description(schema_object),
            )
            .into());
        }

        selections
            .iter()
            .map(|selection| {
                let field_name = selection.name();
                match schema_object.find_field(field_name) {
                    Some(ref field) => self.parse_field(
                        path.clone(),
                        selection_path.clone(),
                        argument_path.clone(),
                        selection,
                        field,
                    ),
                    None => Err(ValidationError::unkown_selection_field(
                        field_name.to_string(),
                        selection_path.segments().clone(),
                        conversions::schema_object_to_output_type_description(schema_object),
                    )
                    .into()),
                }
            })
            .collect::<QueryParserResult<Vec<_>>>()
            .map(|fields| ParsedObject { fields })
    }

    /// Parses and validates a selection against a schema (output) field.
    fn parse_field(
        &self,
        parent_path: QueryPath,
        selection_path: SelectionPath,
        argument_path: ArgumentPath,
        selection: &Selection,
        schema_field: &OutputFieldRef,
    ) -> QueryParserResult<FieldPair> {
        let path = parent_path.add(schema_field.name.clone());
        let selection_path = selection_path.add(schema_field.name.clone());

        // Parse and validate all provided arguments for the field
        self.parse_arguments(
            path.clone(),
            selection_path.clone(),
            argument_path.clone(),
            schema_field,
            selection.arguments(),
        )
        .and_then(|arguments| {
            if !selection.nested_selections().is_empty() && schema_field.field_type.is_scalar() {
                Err(
                    ValidationError::selection_set_on_scalar(selection.name().to_string(), selection_path.segments())
                        .into(),
                )
            } else {
                // If the output type of the field is an object type of any form, validate the sub selection as well.
                let nested_fields = schema_field.field_type.as_object_type().map(|obj| {
                    self.parse_object(
                        path.clone(),
                        selection_path.clone(),
                        argument_path.clone(),
                        selection.nested_selections(),
                        &obj,
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
        parent_path: QueryPath,
        selection_path: SelectionPath,
        argument_path: ArgumentPath,
        schema_field: &OutputFieldRef,
        given_arguments: &[(String, ArgumentValue)],
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
                    conversions::schema_arguments_to_argument_description_vec(&schema_field.arguments),
                )
                .into())
            })
            .collect::<QueryParserResult<Vec<()>>>()?;

        // Check remaining arguments
        schema_field
            .arguments
            .iter()
            .filter_map(|schema_input_arg| {
                // Match schema argument field to an argument field in the incoming document.
                let selection_arg: Option<(String, ArgumentValue)> = given_arguments
                    .iter()
                    .find(|given_argument| given_argument.0 == schema_input_arg.name)
                    .cloned();

                let path = parent_path.add(schema_input_arg.name.clone());
                let argument_path = argument_path.add(schema_input_arg.name.clone());

                // If optional and not present ignore the field.
                // If present, parse normally.
                // If not present but required, throw a validation error.
                match selection_arg {
                    Some((_, value)) => Some(
                        self.parse_input_value(
                            path,
                            selection_path.clone(),
                            argument_path,
                            value,
                            &schema_input_arg.field_types,
                            conversions::schema_output_field_to_input_type_description(schema_field),
                        )
                        .map(|value| ParsedArgument {
                            name: schema_input_arg.name.clone(),
                            value,
                        }),
                    ),

                    None if !schema_input_arg.is_required => None,
                    _ => Some(Err(ValidationError::required_argument_missing(
                        selection_path.segments(),
                        argument_path.segments(),
                        conversions::schema_output_field_to_input_type_description(&schema_field),
                    )
                    .into())),
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
        parent_path: QueryPath,
        selection_path: SelectionPath,
        argument_path: ArgumentPath,
        value: ArgumentValue,
        possible_input_types: &[InputType],
        input_type_description: validation::InputTypeDescription,
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
                        parent_path.segments(),
                        argument_path.segments(),
                        input_type_description.clone(),
                    )
                    .into()),
                    // Scalar handling
                    (value, InputType::Scalar(scalar)) => self
                        .parse_scalar(&selection_path, &argument_path, value, &scalar)
                        .map(ParsedInputValue::Single),

                    // Enum handling
                    (value @ PrismaValue::Enum(_), InputType::Enum(et)) => {
                        self.parse_enum(&selection_path, &argument_path, value, &et.into_arc())
                    }
                    (value @ PrismaValue::String(_), InputType::Enum(et)) => {
                        self.parse_enum(&selection_path, &argument_path, value, &et.into_arc())
                    }
                    (value @ PrismaValue::Boolean(_), InputType::Enum(et)) => {
                        self.parse_enum(&selection_path, &argument_path, value, &et.into_arc())
                    }
                    // Invalid combinations
                    _ => Err(ValidationError::invalid_argument_type(
                        selection_path.segments(),
                        argument_path.segments(),
                        conversions::input_type_to_argument_description(
                            argument_path.last().unwrap_or_default().to_string(),
                            input_type,
                        ),
                    )
                    .into()),
                },

                // List handling.
                (ArgumentValue::List(values), InputType::List(l)) => self
                    .parse_list(
                        &parent_path,
                        &selection_path,
                        &argument_path,
                        values.clone(),
                        &l,
                        input_type_description.clone(),
                    )
                    .map(ParsedInputValue::List),

                // Object handling
                (ArgumentValue::Object(o) | ArgumentValue::FieldRef(o), InputType::Object(obj)) => self
                    .parse_input_object(
                        parent_path.clone(),
                        selection_path.clone(),
                        argument_path.clone(),
                        o.clone(),
                        obj.into_arc(),
                    )
                    .map(ParsedInputValue::Map),

                // Invalid combinations
                _ => Err(ValidationError::invalid_argument_type(
                    selection_path.segments(),
                    argument_path.segments(),
                    conversions::input_type_to_argument_description(
                        argument_path.last().unwrap_or_default().to_string(),
                        input_type,
                    ),
                )
                .into()),
            };

            parse_results.push(result);
        }

        let (successes, mut failures): (Vec<_>, Vec<_>) = parse_results.into_iter().partition(|result| result.is_ok());

        if successes.is_empty() {
            if failures.len() == 1 {
                failures.pop().unwrap()
            } else {
                Err(QueryParserError::Legacy {
                    path: parent_path,
                    error_kind: QueryParserErrorKind::InputUnionParseError {
                        parsing_errors: failures
                            .into_iter()
                            .map(|err| match err {
                                Err(e) => e,
                                Ok(_) => unreachable!("Expecting to only have Result::Err in the `failures` vector."),
                            })
                            .collect(),
                    },
                })
            }
        } else {
            successes.into_iter().next().unwrap()
        }
    }

    /// Attempts to parse given query value into a concrete PrismaValue based on given scalar type.
    fn parse_scalar(
        &self,
        selection_path: &SelectionPath,
        argument_path: &ArgumentPath,
        value: PrismaValue,
        scalar_type: &ScalarType,
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
                )
                .into()),
            },

            // UUID coercion matchers
            (PrismaValue::Uuid(uuid), ScalarType::String) => Ok(PrismaValue::String(uuid.to_string())),

            // All other combinations are value type mismatches.
            (_, _) => Err(ValidationError::invalid_argument_type(
                selection_path.segments(),
                argument_path.segments(),
                conversions::scalar_type_to_argument_description(
                    argument_path.last().unwrap_or_default().to_string(),
                    scalar_type,
                ),
            )
            .into()),
        }
    }

    fn parse_datetime(
        &self,
        selection_path: &SelectionPath,
        argument_path: &ArgumentPath,
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
            .into()
        })
    }

    fn parse_bytes(
        &self,
        selection_path: &SelectionPath,
        argument_path: &ArgumentPath,
        s: String,
    ) -> QueryParserResult<PrismaValue> {
        prisma_value::decode_bytes(&s).map(PrismaValue::Bytes).map_err(|err| {
            ValidationError::invalid_argument_value(
                selection_path.segments(),
                argument_path.segments(),
                s.to_string(),
                String::from("base64 String"),
                Some(Box::new(err)),
            )
            .into()
        })
    }

    fn parse_decimal(
        &self,
        selection_path: &SelectionPath,
        argument_path: &ArgumentPath,
        s: String,
    ) -> QueryParserResult<PrismaValue> {
        BigDecimal::from_str(&s).map(PrismaValue::Float).map_err(|err| {
            ValidationError::invalid_argument_value(
                selection_path.segments(),
                argument_path.segments(),
                s.to_string(),
                String::from("decimal String"),
                Some(Box::new(err)),
            )
            .into()
        })
    }

    fn parse_bigint(
        &self,
        selection_path: &SelectionPath,
        argument_path: &ArgumentPath,
        s: String,
    ) -> QueryParserResult<PrismaValue> {
        s.parse::<i64>().map(PrismaValue::BigInt).map_err(|err| {
            ValidationError::invalid_argument_value(
                selection_path.segments(),
                argument_path.segments(),
                s.to_string(),
                String::from("big integer String"),
                Some(Box::new(err)),
            )
            .into()
        })
    }

    fn parse_json_list_from_str(
        &self,
        selection_path: &SelectionPath,
        argument_path: &ArgumentPath,
        s: &str,
    ) -> QueryParserResult<PrismaValue> {
        let json = self.parse_json(selection_path, argument_path, s)?;
        self.parse_json_list_from_value(selection_path, argument_path, json)
    }

    fn parse_json_list_from_value(
        &self,
        selection_path: &SelectionPath,
        argument_path: &ArgumentPath,
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

    fn parse_json(
        &self,
        selection_path: &SelectionPath,
        argument_path: &ArgumentPath,
        s: &str,
    ) -> QueryParserResult<serde_json::Value> {
        serde_json::from_str(s).map_err(|err| {
            ValidationError::invalid_argument_value(
                selection_path.segments(),
                argument_path.segments(),
                s.to_string(),
                String::from("JSON String"),
                Some(Box::new(err)),
            )
            .into()
        })
    }

    fn to_json(
        &self,
        selection_path: &SelectionPath,
        argument_path: &ArgumentPath,
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
                .into()
            })
            .map(PrismaValue::Json)
    }

    fn parse_uuid(
        &self,
        selection_path: &SelectionPath,
        argument_path: &ArgumentPath,
        s: &str,
    ) -> QueryParserResult<Uuid> {
        Uuid::parse_str(s).map_err(|err| {
            ValidationError::invalid_argument_value(
                selection_path.segments(),
                argument_path.segments(),
                s.to_string(),
                String::from("UUID String"),
                Some(Box::new(err)),
            )
            .into()
        })
    }

    fn parse_list(
        &self,
        path: &QueryPath,
        selection_path: &SelectionPath,
        argument_path: &ArgumentPath,
        values: Vec<ArgumentValue>,
        value_type: &InputType,
        input_type_description: validation::InputTypeDescription,
    ) -> QueryParserResult<Vec<ParsedInputValue>> {
        values
            .into_iter()
            .map(|val| {
                self.parse_input_value(
                    path.clone(),
                    selection_path.clone(),
                    argument_path.clone(),
                    val,
                    &[value_type.clone()],
                    input_type_description.clone(),
                )
            })
            .collect::<QueryParserResult<Vec<ParsedInputValue>>>()
    }

    fn parse_enum(
        &self,
        selection_path: &SelectionPath,
        argument_path: &ArgumentPath,
        val: PrismaValue,
        typ: &EnumTypeRef,
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
                    typ.name().to_string(),
                    None,
                )
                .into());
            }
        };

        let err = |name: &str| {
            Err(ValidationError::invalid_argument_value(
                selection_path.segments(),
                argument_path.segments(),
                raw.clone(),
                name.to_string(),
                None,
            )
            .into())
        };

        match typ.borrow() {
            EnumType::Database(db) => match db.map_input_value(&raw) {
                Some(value) => Ok(ParsedInputValue::Single(value)),
                None => err(db.identifier().name()),
            },
            EnumType::String(s) => match s.value_for(raw.as_str()) {
                Some(val) => Ok(ParsedInputValue::Single(PrismaValue::Enum(val.to_owned()))),
                None => err(s.identifier().name()),
            },
            EnumType::FieldRef(f) => match f.value_for(raw.as_str()) {
                Some(value) => Ok(ParsedInputValue::ScalarField(value.clone())),
                None => err(f.identifier().name()),
            },
        }
    }

    /// Parses and validates an input object recursively.
    fn parse_input_object(
        &self,
        parent_path: QueryPath,
        selection_path: SelectionPath,
        argument_path: ArgumentPath,
        object: ArgumentValueObject,
        schema_object: InputObjectTypeStrongRef,
    ) -> QueryParserResult<ParsedInputMap> {
        let path = parent_path.add(schema_object.identifier.name().to_owned());

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
                let path = path.add(field.name.clone());
                let argument_path = path.add(field.name.clone());

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
                            path,
                            selection_path.clone(),
                            argument_path,
                            default_pv.into(),
                            &field.field_types,
                            conversions::schema_input_object_type_to_input_type_description(&schema_object),
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
                                conversions::schema_input_object_type_to_input_type_description(&schema_object),
                            )
                            .into()))
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
                        path.add(field_name.clone()).segments(),
                        conversions::schema_input_object_type_to_input_type_description(&schema_object),
                    )
                })?;

                let path = path.add(field.name.clone());
                let argument_path = argument_path.add(field.name.clone());
                let parsed = self.parse_input_value(
                    path,
                    selection_path.clone(),
                    argument_path,
                    value,
                    &field.field_types,
                    conversions::schema_input_object_type_to_input_type_description(&schema_object),
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
            )
            .into());
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
            )
            .into());
        }

        map.set_tag(schema_object.tag.clone());

        Ok(map)
    }
}
