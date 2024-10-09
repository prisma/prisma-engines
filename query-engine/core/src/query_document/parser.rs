use super::*;
use crate::{executor::get_engine_protocol, schema::*};
use bigdecimal::{BigDecimal, ToPrimitive};
use chrono::prelude::*;
use core::fmt;
use geojson::Geometry;
use indexmap::{IndexMap, IndexSet};
use query_structure::{DefaultKind, PrismaValue, ValueGeneratorFn};
use std::{borrow::Cow, convert::TryFrom, rc::Rc, str::FromStr};
use user_facing_errors::query_engine::validation::ValidationError;
use uuid::Uuid;

pub(crate) struct QueryDocumentParser {
    /// NOW() default value that's reused for all NOW() defaults on a single query
    default_now: PrismaValue,
}

type ResolveField<'a, 'b> = &'b dyn Fn(&str) -> Option<OutputField<'a>>;

impl QueryDocumentParser {
    pub(crate) fn new(default_now: PrismaValue) -> Self {
        QueryDocumentParser { default_now }
    }

    // Public entry point to parsing the query document (as denoted by `selections`) against the `schema_object`.
    pub fn parse<'a>(
        &self,
        selections: &[Selection],
        exclusions: Option<&[Exclusion]>,
        schema_object: &ObjectType<'a>,
        fields: ResolveField<'a, '_>,
        query_schema: &'a QuerySchema,
    ) -> QueryParserResult<ParsedObject<'a>> {
        self.parse_object(
            Path::default(),
            Path::default(),
            selections,
            exclusions,
            schema_object,
            Some(fields),
            query_schema,
        )
    }

    /// Parses and validates a set of selections against a schema (output) object.
    /// On an output object, optional types designate whether or not an output field can be nulled.
    /// In contrast, nullable and optional types on an input object are separate concepts.
    /// The above is the reason we don't need to check nullability here, as it is done by the output
    /// validation in the serialization step.
    #[allow(clippy::too_many_arguments)]
    fn parse_object<'a>(
        &self,
        selection_path: Path,
        argument_path: Path,
        selections: &[Selection],
        exclusions: Option<&[Exclusion]>,
        schema_object: &ObjectType<'a>,
        resolve_field: Option<ResolveField<'a, '_>>,
        query_schema: &'a QuerySchema,
    ) -> QueryParserResult<ParsedObject<'a>> {
        if selections.is_empty() {
            return Err(ValidationError::empty_selection(
                selection_path.segments(),
                conversions::schema_object_to_output_type_description(schema_object),
            ));
        }

        let resolve_adhoc = move |name: &str| schema_object.find_field(name).cloned();
        let resolve_field = resolve_field.unwrap_or(&resolve_adhoc);

        if let Some(exclusions) = exclusions {
            for exclusion in exclusions {
                if resolve_field(&exclusion.name).is_none() {
                    return Err(ValidationError::unknown_selection_field(
                        selection_path.add(exclusion.name.to_owned()).segments(),
                        conversions::schema_object_to_output_type_description(schema_object),
                    ));
                }
            }
        }

        selections
            .iter()
            .map(|selection| {
                let field_name = selection.name();
                match resolve_field(field_name) {
                    Some(field) => self.parse_field(
                        selection_path.clone(),
                        argument_path.clone(),
                        selection,
                        field,
                        query_schema,
                    ),
                    None => Err(ValidationError::unknown_selection_field(
                        selection_path.add(field_name.to_owned()).segments(),
                        conversions::schema_object_to_output_type_description(schema_object),
                    )),
                }
            })
            .collect::<QueryParserResult<Vec<_>>>()
            .map(|fields| ParsedObject { fields })
    }

    /// Parses and validates a selection against a schema (output) field.
    fn parse_field<'a>(
        &self,
        selection_path: Path,
        argument_path: Path,
        selection: &Selection,
        schema_field: OutputField<'a>,
        query_schema: &'a QuerySchema,
    ) -> QueryParserResult<FieldPair<'a>> {
        let selection_path = selection_path.add(schema_field.name().clone().into_owned());

        // Parse and validate all provided arguments for the field
        self.parse_arguments(
            selection_path.clone(),
            argument_path.clone(),
            &schema_field,
            selection.arguments(),
            query_schema,
        )
        .and_then(move |arguments| {
            if !selection.nested_selections().is_empty() && schema_field.field_type().is_scalar() {
                Err(ValidationError::selection_set_on_scalar(
                    selection.name().to_string(),
                    selection_path.segments(),
                ))
            } else {
                // If the output type of the field is an object type of any form, validate the sub selection as well.
                let nested_fields = schema_field.field_type().as_object_type().map(|obj| {
                    self.parse_object(
                        selection_path.clone(),
                        argument_path.clone(),
                        selection.nested_selections(),
                        selection.nested_exclusions(),
                        obj,
                        None,
                        query_schema,
                    )
                });

                let nested_fields = match nested_fields {
                    Some(sub) => Some(sub?),
                    None => None,
                };

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
    fn parse_arguments<'a>(
        &self,
        selection_path: Path,
        argument_path: Path,
        schema_field: &OutputField<'a>,
        given_arguments: &[(String, ArgumentValue)],
        query_schema: &'a QuerySchema,
    ) -> QueryParserResult<Vec<ParsedArgument<'a>>> {
        for (name, _) in given_arguments {
            if !schema_field.arguments().iter().any(|arg| arg.name == name.as_str()) {
                let argument_path = argument_path.add(name.clone());
                return Err(ValidationError::unknown_argument(
                    selection_path.segments(),
                    argument_path.segments(),
                    conversions::schema_arguments_to_argument_description_vec(schema_field.arguments().iter().cloned()),
                ));
            }
        }

        // Check remaining arguments
        schema_field
            .arguments()
            .iter()
            .filter_map(|input_field| {
                // Match schema argument field to an argument field in the incoming document.
                let selection_arg: Option<(String, ArgumentValue)> = given_arguments
                    .iter()
                    .find(|given_argument| given_argument.0 == input_field.name)
                    .cloned();

                let argument_path = argument_path.add(input_field.name.clone().into_owned());

                // If optional and not present ignore the field.
                // If present, parse normally.
                // If not present but required, throw a validation error.
                match selection_arg {
                    Some((_, value)) => Some(
                        self.parse_input_value(
                            selection_path.clone(),
                            argument_path,
                            value,
                            input_field.field_types(),
                            query_schema,
                        )
                        .map(|value| ParsedArgument {
                            name: input_field.name.clone().into_owned(),
                            value,
                        }),
                    ),
                    None if !input_field.is_required() => None,
                    _ => Some(Err(ValidationError::required_argument_missing(
                        selection_path.segments(),
                        argument_path.segments(),
                        &conversions::input_types_to_input_type_descriptions(input_field.field_types()),
                    ))),
                }
            })
            .collect::<Vec<QueryParserResult<ParsedArgument<'_>>>>()
            .into_iter()
            .collect()
    }

    /// Parses and validates an ArgumentValue against possible input types.
    /// Matching is done in order of definition on the input type. First matching type wins.
    fn parse_input_value<'a>(
        &self,
        selection_path: Path,
        argument_path: Path,
        value: ArgumentValue,
        possible_input_types: &[InputType<'a>],
        query_schema: &'a QuerySchema,
    ) -> QueryParserResult<ParsedInputValue<'a>> {
        let mut failures = Vec::new();

        macro_rules! try_this {
            ($e:expr) => {
                match $e {
                    success @ Ok(_) => return success,
                    Err(failure) => {
                        failures.push(failure);
                    }
                }
            };
        }

        for input_type in possible_input_types {
            match (value.clone(), input_type) {
                // With the JSON protocol, JSON values are sent as deserialized values.
                // This means JSON can match with pretty much anything. A string, an int, an object, an array.
                // This is an early catch-all.
                // We do not get into this catch-all _if_ the value is already Json, if it's a FieldRef or if it's an Enum.
                // We don't because they've already been desambiguified at the procotol adapter level.
                (value, InputType::<'a>::Scalar(ScalarType::Json))
                    if value.should_be_parsed_as_json() && get_engine_protocol().is_json() =>
                {
                    return Ok(ParsedInputValue::Single(self.to_json(
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
                            "JSON array",
                            Some(Box::new(err)),
                        )
                    })?;
                    let json_list = self.parse_json_list_from_value(&selection_path, &argument_path, json_val)?;

                    return Ok(ParsedInputValue::Single(json_list));
                }
                (ArgumentValue::Scalar(pv), input_type) => match (pv, input_type) {
                    // Null handling
                    (PrismaValue::Null, InputType::Scalar(ScalarType::Null)) => {
                        return Ok(ParsedInputValue::Single(PrismaValue::Null))
                    }
                    (PrismaValue::Null, input_type) => try_this!(Err(ValidationError::required_argument_missing(
                        selection_path.segments(),
                        argument_path.segments(),
                        &conversions::input_types_to_input_type_descriptions(&[input_type.clone()],),
                    ))),
                    // Scalar handling
                    (pv, InputType::Scalar(st)) => try_this!(self
                        .parse_scalar(&selection_path, &argument_path, pv, *st, &value)
                        .map(ParsedInputValue::Single)),

                    // Enum handling
                    (pv @ PrismaValue::Enum(_), InputType::Enum(et)) => {
                        try_this!(self.parse_enum(&selection_path, &argument_path, pv, et))
                    }
                    (pv @ PrismaValue::String(_), InputType::Enum(et)) => {
                        try_this!(self.parse_enum(&selection_path, &argument_path, pv, et))
                    }
                    (pv @ PrismaValue::Boolean(_), InputType::Enum(et)) => {
                        try_this!(self.parse_enum(&selection_path, &argument_path, pv, et))
                    }
                    // Invalid combinations
                    (_, input_type) => try_this!(Err(ValidationError::invalid_argument_type(
                        selection_path.segments(),
                        argument_path.segments(),
                        conversions::input_type_to_argument_description(
                            argument_path.last().unwrap_or_default(),
                            input_type,
                        ),
                        conversions::argument_value_to_type_name(&value),
                    ))),
                },

                // List handling.
                (ArgumentValue::List(values), InputType::List(l)) => try_this!(self
                    .parse_list(&selection_path, &argument_path, values.clone(), l, query_schema)
                    .map(ParsedInputValue::List)),

                // Object handling
                (ArgumentValue::Object(o) | ArgumentValue::FieldRef(o), InputType::Object(obj)) => try_this!(self
                    .parse_input_object(
                        selection_path.clone(),
                        argument_path.clone(),
                        o.clone(),
                        obj,
                        query_schema,
                    )
                    .map(ParsedInputValue::Map)),

                // Invalid combinations
                (_, input_type) => try_this!(Err(ValidationError::invalid_argument_type(
                    selection_path.segments(),
                    argument_path.segments(),
                    conversions::input_type_to_argument_description(
                        argument_path.last().unwrap_or_default(),
                        input_type,
                    ),
                    conversions::argument_value_to_type_name(&value),
                ))),
            };
        }

        match failures.len() {
            0 => unreachable!("No success and no failure in query document parser."),
            1 => Err(failures.into_iter().next().unwrap()),
            _ => Err(ValidationError::union(failures.into_iter().collect())),
        }
    }

    /// Attempts to parse given query value into a concrete PrismaValue based on given scalar type.
    fn parse_scalar(
        &self,
        selection_path: &Path,
        argument_path: &Path,
        value: PrismaValue,
        scalar_type: ScalarType,
        argument_value: &ArgumentValue,
    ) -> QueryParserResult<PrismaValue> {
        match (value, scalar_type) {
            // Identity matchers
            (PrismaValue::String(s), ScalarType::String) => Ok(PrismaValue::String(s)),
            (PrismaValue::Boolean(b), ScalarType::Boolean) => Ok(PrismaValue::Boolean(b)),
            (PrismaValue::Json(json), ScalarType::Json) => Ok(PrismaValue::Json(json)),
            (PrismaValue::Uuid(uuid), ScalarType::UUID) => Ok(PrismaValue::Uuid(uuid)),
            (PrismaValue::Bytes(bytes), ScalarType::Bytes) => Ok(PrismaValue::Bytes(bytes)),
            (PrismaValue::BigInt(b_int), ScalarType::BigInt) => Ok(PrismaValue::BigInt(b_int)),
            (PrismaValue::DateTime(s), ScalarType::DateTime) => Ok(PrismaValue::DateTime(s)),
            (PrismaValue::Json(s), ScalarType::Geometry) => Ok(PrismaValue::Json(s)),
            (PrismaValue::Null, ScalarType::Null) => Ok(PrismaValue::Null),

            // String coercion matchers
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
            (PrismaValue::String(s), ScalarType::Geometry) => Ok(PrismaValue::Json(
                self.parse_geojson(selection_path, argument_path, &s).map(|_| s)?,
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
                    argument_path.last().unwrap_or_default(),
                    &InputType::Scalar(scalar_type),
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
        query_structure::parse_datetime(s).map_err(|err| {
            ValidationError::invalid_argument_value(
                selection_path.segments(),
                argument_path.segments(),
                s.to_string(),
                "ISO-8601 DateTime",
                Some(Box::new(err)),
            )
        })
    }

    fn parse_bytes(&self, selection_path: &Path, argument_path: &Path, s: String) -> QueryParserResult<PrismaValue> {
        query_structure::decode_bytes(&s)
            .map(PrismaValue::Bytes)
            .map_err(|err| {
                ValidationError::invalid_argument_value(
                    selection_path.segments(),
                    argument_path.segments(),
                    s.to_string(),
                    "base64 String",
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
                "decimal String",
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
                "big integer String",
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
                "JSON array",
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
                    "Flat JSON array (no nesting)",
                    Some(Box::new(err)),
                )
            })?;

            prisma_values.push(pv);
        }

        Ok(PrismaValue::List(prisma_values))
    }

    fn parse_geojson(&self, selection_path: &Path, argument_path: &Path, s: &str) -> QueryParserResult<Geometry> {
        s.parse::<Geometry>().map_err(|err| {
            ValidationError::invalid_argument_value(
                selection_path.segments(),
                argument_path.segments(),
                s.to_string(),
                "GeoJSON String",
                Some(Box::new(err)),
            )
        })
    }

    fn parse_json(&self, selection_path: &Path, argument_path: &Path, s: &str) -> QueryParserResult<serde_json::Value> {
        serde_json::from_str(s).map_err(|err| {
            ValidationError::invalid_argument_value(
                selection_path.segments(),
                argument_path.segments(),
                s.to_string(),
                "JSON String",
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
                    "JSON String",
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
                "UUID String",
                Some(Box::new(err)),
            )
        })
    }

    fn parse_list<'a>(
        &self,
        selection_path: &Path,
        argument_path: &Path,
        values: Vec<ArgumentValue>,
        value_type: &InputType<'a>,
        query_schema: &'a QuerySchema,
    ) -> QueryParserResult<Vec<ParsedInputValue<'a>>> {
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
            .collect::<QueryParserResult<Vec<ParsedInputValue<'a>>>>()
    }

    fn parse_enum<'a>(
        &self,
        selection_path: &Path,
        argument_path: &Path,
        val: PrismaValue,
        typ: &EnumType,
    ) -> QueryParserResult<ParsedInputValue<'a>> {
        let raw = match val {
            PrismaValue::Enum(s) => s,
            PrismaValue::String(s) => s,
            PrismaValue::Boolean(b) => if b { "true" } else { "false" }.to_owned(), // Case where a bool was misinterpreted as constant literal
            _ => {
                return Err(ValidationError::invalid_argument_value(
                    selection_path.segments(),
                    argument_path.segments(),
                    format!("{val:?}"),
                    &typ.name(),
                    None,
                ));
            }
        };

        let err = |name: &str| {
            Err(ValidationError::invalid_argument_value(
                selection_path.segments(),
                argument_path.segments(),
                raw.clone(),
                name,
                None,
            ))
        };

        match typ {
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
    fn parse_input_object<'a>(
        &self,
        selection_path: Path,
        argument_path: Path,
        object: ArgumentValueObject,
        schema_object: &InputObjectType<'a>,
        query_schema: &'a QuerySchema,
    ) -> QueryParserResult<ParsedInputMap<'a>> {
        let fields = schema_object.get_fields().iter();
        let valid_field_names: IndexSet<Cow<'_, str>> = fields.clone().map(|field| field.name.clone()).collect();
        let given_field_names: IndexSet<Cow<'_, str>> = object.iter().map(|(k, _)| Cow::Borrowed(k.as_str())).collect();
        let missing_field_names = valid_field_names.difference(&given_field_names);
        let schema_fields: IndexMap<Cow<'_, str>, InputField<'_>> =
            fields.map(|f| (f.name.clone(), f.clone())).collect();

        // First, filter-in those fields that are not given but have a default value in the schema.
        // As in practise, it is like if they were given with said default value.
        let defaults = missing_field_names
            .filter_map(|unset_field_name| {
                let field = schema_fields.get(unset_field_name.as_ref()).unwrap();
                let argument_path = argument_path.add(field.name.clone().into_owned());

                // If the input field has a default, add the default to the result.
                // If it's not optional and has no default, a required field has not been provided.
                match &field.default_value {
                    Some(default_value) => {
                        let default_pv = match &default_value {
                            DefaultKind::Expression(ref expr) if matches!(expr.generator(), ValueGeneratorFn::Now) => {
                                self.default_now.clone()
                            }
                            _ => default_value.get()?,
                        };

                        match self.parse_input_value(
                            selection_path.clone(),
                            argument_path,
                            default_pv.into(),
                            field.field_types(),
                            query_schema,
                        ) {
                            Ok(value) => Some(Ok((field.name.clone(), value))),
                            Err(err) => Some(Err(err)),
                        }
                    }
                    None => {
                        if field.is_required() {
                            Some(Err(ValidationError::required_argument_missing(
                                selection_path.segments(),
                                argument_path.segments(),
                                &conversions::input_types_to_input_type_descriptions(field.field_types()),
                            )))
                        } else {
                            None
                        }
                    }
                }
            })
            .collect::<QueryParserResult<Vec<(_, ParsedInputValue<'a>)>>>()?;

        // Checks all fields on the provided input object. This will catch extra
        // or unknown fields and parsing errors.
        let mut map = object
            .into_iter()
            .map(|(field_name, value)| {
                let field = schema_fields.get(field_name.as_str()).ok_or_else(|| {
                    ValidationError::unknown_input_field(
                        selection_path.segments(),
                        argument_path.add(field_name.clone()).segments(),
                        conversions::schema_input_object_type_to_input_type_description(schema_object),
                    )
                })?;

                let argument_path = argument_path.add(field.name.clone().into_owned());
                let parsed = self.parse_input_value(
                    selection_path.clone(),
                    argument_path,
                    value,
                    field.field_types(),
                    query_schema,
                )?;

                Ok((Cow::Owned(field_name), parsed))
            })
            .collect::<QueryParserResult<ParsedInputMap<'a>>>()?;

        map.extend(defaults);

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
                &conversions::schema_input_object_type_to_input_type_description(schema_object),
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
                &conversions::schema_input_object_type_to_input_type_description(schema_object),
            ));
        }

        map.set_tag(schema_object.tag().cloned());

        Ok(map)
    }
}

pub(crate) mod conversions {
    use std::borrow::Cow;

    use crate::{
        schema::{InputType, OutputType},
        ArgumentValue,
    };
    use query_structure::PrismaValue;
    use schema::InnerOutputType;
    use user_facing_errors::query_engine::validation::{self, InputTypeDescription};

    /// Converts an schema object to the narrower validation::OutputTypeDescription
    /// representation of an output field that is part of a validation error information.
    pub(crate) fn schema_object_to_output_type_description(
        o: &schema::ObjectType<'_>,
    ) -> validation::OutputTypeDescription {
        let name = o.name();
        let fields: Vec<validation::OutputTypeDescriptionField> = o
            .get_fields()
            .iter()
            .map(|field| {
                let name = field.name();
                let type_name = to_simplified_output_type_name(field.field_type());
                let is_relation = field.maps_to_relation();

                validation::OutputTypeDescriptionField::new(name.clone().into_owned(), type_name, is_relation)
            })
            .collect();
        validation::OutputTypeDescription::new(name, fields)
    }

    pub(crate) fn input_types_to_input_type_descriptions(
        input_types: &[schema::InputType<'_>],
    ) -> Vec<validation::InputTypeDescription> {
        input_types.iter().map(input_type_to_input_type_description).collect()
    }

    fn input_type_to_input_type_description(input_type: &InputType<'_>) -> InputTypeDescription {
        match input_type {
            InputType::Scalar(s) => InputTypeDescription::Scalar { name: s.to_string() },
            InputType::Enum(e) => InputTypeDescription::Enum { name: e.name() },
            InputType::List(l) => InputTypeDescription::List {
                element_type: Box::new(input_type_to_input_type_description(l.as_ref())),
            },
            InputType::Object(object) => schema_input_object_type_to_input_type_description(object),
        }
    }

    pub(crate) fn schema_input_object_type_to_input_type_description(
        i: &schema::InputObjectType<'_>,
    ) -> validation::InputTypeDescription {
        let name = i.identifier.name();
        let fields: Vec<validation::InputTypeDescriptionField> = i
            .get_fields()
            .iter()
            .map(|field| {
                let name = field.name.clone();
                let type_names: Vec<_> = field.field_types().iter().map(to_simplified_input_type_name).collect();
                validation::InputTypeDescriptionField::new(name.clone().into_owned(), type_names, field.is_required())
            })
            .collect();
        validation::InputTypeDescription::new_object(name, fields)
    }

    pub(crate) fn schema_arguments_to_argument_description_vec<'a>(
        arguments: impl Iterator<Item = schema::InputField<'a>>,
    ) -> Vec<validation::ArgumentDescription<'a>> {
        arguments
            .map(|input_field_ref| {
                let type_names = input_field_ref
                    .field_types()
                    .iter()
                    .map(|t| Cow::Owned(to_simplified_input_type_name(t)))
                    .collect();
                validation::ArgumentDescription::new(input_field_ref.name, type_names)
            })
            .collect::<Vec<_>>()
    }

    pub(crate) fn input_type_to_argument_description<'a>(
        arg_name: &'a str,
        input_type: &InputType<'_>,
    ) -> validation::ArgumentDescription<'a> {
        validation::ArgumentDescription::new(arg_name, vec![Cow::Owned(to_simplified_input_type_name(input_type))])
    }

    pub(crate) fn argument_value_to_type_name(value: &ArgumentValue) -> String {
        match value {
            ArgumentValue::Scalar(pv) => prisma_value_to_type_name(pv),
            ArgumentValue::Object(_) => "Object".to_string(),
            ArgumentValue::List(v) => {
                format!("({})", itertools::join(v.iter().map(argument_value_to_type_name), ", "))
            }
            ArgumentValue::FieldRef(_) => "FieldRef".to_string(),
            ArgumentValue::Raw(_) => "JSON".to_string(),
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
            PrismaValue::Object(_) => "Object".to_string(),
            PrismaValue::Null => "Null".to_string(),
            PrismaValue::DateTime(_) => "DateTime".to_string(),
            PrismaValue::Float(_) => "Float".to_string(),
            PrismaValue::BigInt(_) => "BigInt".to_string(),
            PrismaValue::Bytes(_) => "Bytes".to_string(),
        }
    }

    fn to_simplified_input_type_name(typ: &InputType<'_>) -> String {
        match typ {
            InputType::Enum(e) => e.name(),
            InputType::List(o) => format!("{}[]", to_simplified_input_type_name(o.as_ref(),)),
            InputType::Object(object) => object.identifier.name(),
            InputType::Scalar(s) => s.to_string(),
        }
    }

    fn to_simplified_output_type_name(typ: &OutputType<'_>) -> String {
        if typ.is_list() {
            return format!(
                "{}[]",
                to_simplified_output_type_name(&OutputType::non_list(typ.inner.clone()))
            );
        }

        match &typ.inner {
            InnerOutputType::Enum(e) => e.name(),
            InnerOutputType::Object(o) => o.name(),
            InnerOutputType::Scalar(s) => s.to_string(),
        }
    }
}
#[derive(Debug, Clone, Default)]
pub(crate) struct Path {
    next: Rc<Option<(String, Path)>>,
}

impl Path {
    pub(crate) fn add(&self, segment: String) -> Self {
        Path {
            next: Rc::new(Some((segment, self.clone()))),
        }
    }

    pub(crate) fn last(&self) -> Option<&str> {
        Some(&self.next.as_ref().as_ref()?.0)
    }

    pub(crate) fn segments(&self) -> Vec<&str> {
        let mut out = Vec::new();
        let mut cur = &self.next;
        while let Some((segment, next)) = cur.as_ref() {
            out.push(segment.as_str());
            cur = &next.next;
        }
        out.reverse();
        out
    }
}

impl fmt::Display for Path {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.segments().join("."))
    }
}
