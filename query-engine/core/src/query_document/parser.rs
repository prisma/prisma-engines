use super::*;
use crate::schema::*;
use chrono::prelude::*;
use indexmap::IndexMap;
use prisma_value::PrismaValue;
use rust_decimal::{prelude::ToPrimitive, Decimal};
use std::{borrow::Borrow, collections::HashSet, convert::TryFrom, sync::Arc};
use uuid::Uuid;

// todo: validate is one of!

pub struct QueryDocumentParser;

// Todo:
// - Use error collections instead of letting first error win.
// - UUID ids are not encoded in any useful way in the schema.
impl QueryDocumentParser {
    /// Parses and validates a set of selections against a schema (output) object.
    /// On an output object, optional types designate whether or not an output field can be nulled.
    /// In contrast, nullable and optional types on an input object are separate concepts.
    /// The above is the reason we don't need to check nullability here, as it is done by the output
    /// validation in the serialization step.
    pub fn parse_object(
        parent_path: QueryPath,
        selections: &[Selection],
        schema_object: &ObjectTypeStrongRef,
    ) -> QueryParserResult<ParsedObject> {
        let path = parent_path.add(schema_object.name().to_string());

        // Basic invariant not (yet) encoded in the schema: Output objects can't be empty.
        if selections.is_empty() {
            return Err(QueryParserError {
                path,
                error_kind: QueryParserErrorKind::FieldCountError(FieldCountError::new(Some(1), None, 0)),
            });
        }

        selections
            .iter()
            .map(|selection| match schema_object.find_field(selection.name()) {
                Some(ref field) => Self::parse_field(path.clone(), selection, field),
                None => Err(QueryParserError {
                    path: path.add(selection.name().into()),
                    error_kind: QueryParserErrorKind::FieldNotFoundError,
                }),
            })
            .collect::<QueryParserResult<Vec<ParsedField>>>()
            .map(|fields| ParsedObject { fields })
    }

    /// Parses and validates a selection against a schema (output) field.
    fn parse_field(
        parent_path: QueryPath,
        selection: &Selection,
        schema_field: &OutputFieldRef,
    ) -> QueryParserResult<ParsedField> {
        let path = parent_path.add(schema_field.name.clone());

        // Parse and validate all provided arguments for the field
        Self::parse_arguments(path.clone(), schema_field, selection.arguments()).and_then(|arguments| {
            // If the output type of the field is an object type of any form, validate the sub selection as well.
            let nested_fields = schema_field
                .field_type
                .as_object_type()
                .map(|obj| Self::parse_object(path.clone(), selection.nested_selections(), &obj));

            let nested_fields = match nested_fields {
                Some(sub) => Some(sub?),
                None => None,
            };

            Ok(ParsedField {
                name: selection.name().to_string(),
                alias: selection.alias().clone(),
                arguments,
                nested_fields,
                schema_field: Arc::clone(schema_field),
            })
        })
    }

    /// Parses and validates selection arguments against a schema defined field.
    pub fn parse_arguments(
        parent_path: QueryPath,
        schema_field: &OutputFieldRef,
        given_arguments: &[(String, QueryValue)],
    ) -> QueryParserResult<Vec<ParsedArgument>> {
        let left: HashSet<&str> = schema_field.arguments.iter().map(|arg| arg.name.as_str()).collect();
        let right: HashSet<&str> = given_arguments.iter().map(|arg| arg.0.as_str()).collect();
        let diff = Diff::new(&left, &right);

        // All arguments that are not in the schema cause an error.
        diff.right
            .into_iter()
            .map(|extra_arg| {
                Err(QueryParserError {
                    path: parent_path.add(extra_arg.to_string()),
                    error_kind: QueryParserErrorKind::ArgumentNotFoundError,
                })
            })
            .collect::<QueryParserResult<Vec<()>>>()?;

        // Check remaining arguments
        schema_field
            .arguments
            .iter()
            .filter_map(|schema_input_arg| {
                // Match schema argument field to an argument field in the incoming document.
                let selection_arg: Option<(String, QueryValue)> = given_arguments
                    .iter()
                    .find(|given_argument| given_argument.0 == schema_input_arg.name)
                    .cloned();

                let path = parent_path.add(schema_input_arg.name.clone());

                // If optional and not present ignore the field.
                // If present, parse normally.
                // If not present but required, throw a validation error.
                match selection_arg {
                    Some((_, value)) => Some(
                        Self::parse_input_value(path.clone(), value, &schema_input_arg.field_types).map(|value| {
                            ParsedArgument {
                                name: schema_input_arg.name.clone(),
                                value,
                            }
                        }),
                    ),

                    None if !schema_input_arg.is_required => None,
                    _ => Some(Err(QueryParserError {
                        path,
                        error_kind: QueryParserErrorKind::RequiredValueNotSetError,
                    })),
                }
            })
            .collect::<Vec<QueryParserResult<ParsedArgument>>>()
            .into_iter()
            .collect()
    }

    /// Parses and validates a QueryValue against possible input types.
    /// Matching is done in order of definition on the input type. First matching type wins.
    pub fn parse_input_value(
        parent_path: QueryPath,
        value: QueryValue,
        possible_input_types: &[InputType],
    ) -> QueryParserResult<ParsedInputValue> {
        let mut parse_results = vec![];

        for input_type in possible_input_types {
            let value = value.clone();
            let result = match (&value, input_type) {
                // Null handling
                (QueryValue::Null, InputType::Scalar(ScalarType::Null)) => {
                    Ok(ParsedInputValue::Single(PrismaValue::Null))
                }
                (QueryValue::Null, _) => Err(QueryParserError {
                    path: parent_path.clone(),
                    error_kind: QueryParserErrorKind::RequiredValueNotSetError,
                }),

                // Scalar handling
                (_, InputType::Scalar(scalar)) => {
                    Self::parse_scalar(&parent_path, value, &scalar).map(ParsedInputValue::Single)
                }

                // Enum handling
                (QueryValue::Enum(_), InputType::Enum(et)) => Self::parse_enum(&parent_path, value, et),
                (QueryValue::String(_), InputType::Enum(et)) => Self::parse_enum(&parent_path, value, et),
                (QueryValue::Boolean(_), InputType::Enum(et)) => Self::parse_enum(&parent_path, value, et),

                // List handling.
                (QueryValue::List(values), InputType::List(l)) => {
                    Self::parse_list(&parent_path, values.clone(), &l).map(ParsedInputValue::List)
                }

                // Object handling
                (QueryValue::Object(o), InputType::Object(obj)) => {
                    Self::parse_input_object(parent_path.clone(), o.clone(), obj.into_arc()).map(ParsedInputValue::Map)
                }

                // Invalid combinations
                _ => Err(QueryParserError {
                    path: parent_path.clone(),
                    error_kind: QueryParserErrorKind::ValueTypeMismatchError {
                        have: value,
                        want: input_type.clone(),
                    },
                }),
            };

            parse_results.push(result);
        }

        let (successes, mut failures): (Vec<_>, Vec<_>) = parse_results.into_iter().partition(|result| result.is_ok());
        if successes.is_empty() {
            if failures.len() == 1 {
                failures.pop().unwrap()
            } else {
                Err(QueryParserError {
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
    pub fn parse_scalar(
        parent_path: &QueryPath,
        value: QueryValue,
        scalar_type: &ScalarType,
    ) -> QueryParserResult<PrismaValue> {
        match (value, scalar_type.clone()) {
            (QueryValue::String(s), ScalarType::String) => Ok(PrismaValue::String(s)),
            (QueryValue::String(s), ScalarType::DateTime) => {
                Self::parse_datetime(parent_path, s.as_str()).map(PrismaValue::DateTime)
            }
            (QueryValue::String(s), ScalarType::Json) => {
                Ok(PrismaValue::Json(Self::parse_json(parent_path, &s).map(|_| s)?))
            }
            (QueryValue::String(s), ScalarType::JsonList) => Self::parse_json_list(parent_path, &s),
            (QueryValue::String(s), ScalarType::UUID) => {
                Self::parse_uuid(parent_path, s.as_str()).map(PrismaValue::Uuid)
            }
            (QueryValue::Int(i), ScalarType::Float) => Ok(PrismaValue::Float(Decimal::from(i))),
            (QueryValue::Int(i), ScalarType::Int) => Ok(PrismaValue::Int(i)),
            (QueryValue::Float(f), ScalarType::Float) => Ok(PrismaValue::Float(f)),
            (QueryValue::Float(f), ScalarType::Int) => Ok(PrismaValue::Int(f.to_i64().unwrap())),
            (QueryValue::Boolean(b), ScalarType::Boolean) => Ok(PrismaValue::Boolean(b)),

            // All other combinations are value type mismatches.
            (qv, _) => Err(QueryParserError {
                path: parent_path.clone(),
                error_kind: QueryParserErrorKind::ValueTypeMismatchError {
                    have: qv,
                    want: InputType::Scalar(scalar_type.clone()),
                },
            }),
        }
    }

    pub fn parse_datetime(path: &QueryPath, s: &str) -> QueryParserResult<DateTime<Utc>> {
        let fmt = "%Y-%m-%dT%H:%M:%S%.3f";
        Utc.datetime_from_str(s.trim_end_matches('Z'), fmt)
            .map(|dt| DateTime::<Utc>::from_utc(dt.naive_utc(), Utc))
            .map_err(|err| QueryParserError {
                path: path.clone(),
                error_kind: QueryParserErrorKind::ValueParseError(format!(
                    "Invalid DateTime: {} DateTime must adhere to format: %Y-%m-%dT%H:%M:%S%.3f",
                    err
                )),
            })
    }

    // [DTODO] This is likely incorrect or at least using the wrong abstractions.
    pub fn parse_json_list(path: &QueryPath, s: &str) -> QueryParserResult<PrismaValue> {
        let json = Self::parse_json(path, s)?;

        let values = json.as_array().ok_or_else(|| QueryParserError {
            path: path.clone(),
            error_kind: QueryParserErrorKind::AssertionError("JSON parameter needs to be an array".into()),
        })?;

        let mut prisma_values = Vec::with_capacity(values.len());

        for v in values.into_iter() {
            let pv = PrismaValue::try_from(v.clone()).map_err(|_| QueryParserError {
                path: path.clone(),
                error_kind: QueryParserErrorKind::AssertionError("Nested JSON arguments are not supported".into()),
            })?;

            prisma_values.push(pv);
        }

        Ok(PrismaValue::List(prisma_values))
    }

    pub fn parse_json(path: &QueryPath, s: &str) -> QueryParserResult<serde_json::Value> {
        serde_json::from_str(s).map_err(|err| QueryParserError {
            path: path.clone(),
            error_kind: QueryParserErrorKind::ValueParseError(format!("Invalid json: {}", err)),
        })
    }

    pub fn parse_uuid(path: &QueryPath, s: &str) -> QueryParserResult<Uuid> {
        Uuid::parse_str(s).map_err(|err| QueryParserError {
            path: path.clone(),
            error_kind: QueryParserErrorKind::ValueParseError(format!("Invalid UUID: {}", err)),
        })
    }

    pub fn parse_list(
        path: &QueryPath,
        values: Vec<QueryValue>,
        value_type: &InputType,
    ) -> QueryParserResult<Vec<ParsedInputValue>> {
        values
            .into_iter()
            .map(|val| Self::parse_input_value(path.clone(), val, &[value_type.clone()]))
            .collect::<QueryParserResult<Vec<ParsedInputValue>>>()
    }

    pub fn parse_enum(path: &QueryPath, val: QueryValue, typ: &EnumTypeRef) -> QueryParserResult<ParsedInputValue> {
        let raw = match val {
            QueryValue::Enum(s) => s,
            QueryValue::String(s) => s,
            QueryValue::Boolean(b) => if b { "true" } else { "false" }.to_owned(), // Case where a bool was misinterpreted as constant literal
            _ => {
                return Err(QueryParserError {
                    path: path.clone(),
                    error_kind: QueryParserErrorKind::ValueParseError(format!(
                        "Unexpected Enum value type {:?} for enum {}",
                        val,
                        typ.name()
                    )),
                });
            }
        };

        let err = |name: &str| {
            Err(QueryParserError {
                path: path.clone(),
                error_kind: QueryParserErrorKind::ValueParseError(format!(
                    "Enum value '{}' is invalid for enum type {}",
                    raw, name
                )),
            })
        };

        match typ.borrow() {
            EnumType::Internal(i) => match i.map_input_value(&raw) {
                Some(value) => Ok(ParsedInputValue::Single(value)),
                None => err(&i.name),
            },
            EnumType::String(s) => match s.value_for(raw.as_str()) {
                Some(val) => Ok(ParsedInputValue::Single(PrismaValue::String(val.to_owned()))),
                None => err(&s.name),
            },
            EnumType::FieldRef(f) => match f.value_for(raw.as_str()) {
                Some(value) => Ok(ParsedInputValue::ScalarField(value.clone())),
                None => err(&f.name),
            },
        }
    }

    /// Parses and validates an input object recursively.
    pub fn parse_input_object(
        parent_path: QueryPath,
        object: IndexMap<String, QueryValue>,
        schema_object: InputObjectTypeStrongRef,
    ) -> QueryParserResult<ParsedInputMap> {
        let path = parent_path.add(schema_object.name.clone());
        let left: HashSet<&str> = schema_object
            .get_fields()
            .iter()
            .map(|field| field.name.as_str())
            .collect();

        let right: HashSet<&str> = object.keys().map(|k| k.as_str()).collect();
        let diff = Diff::new(&left, &right);

        // First, check that all fields **not** provided in the query (left diff) are optional,
        // i.e. run the validation but disregard the result, or have defaults, in which case the
        // value pair gets added to the result.
        diff.left
            .into_iter()
            .filter_map(|unset_field_name| {
                let field = schema_object.find_field(*unset_field_name).unwrap();
                let path = path.add(field.name.clone());
                let default_pair = field.default_value.clone().map(|def| (&field.name, def));

                // If the input field has a default, add the default to the result.
                // If it's not optional and has no default, a required field has not been provided.
                match default_pair {
                    Some((k, dv)) => {
                        dv.get().map(
                            |pv| match Self::parse_input_value(path, pv.into(), &field.field_types) {
                                Ok(value) => Ok((k.clone(), value)),
                                Err(err) => Err(err),
                            },
                        )
                    }

                    None if field.is_required => Some(Err(QueryParserError {
                        path: path.clone(),
                        error_kind: QueryParserErrorKind::RequiredValueNotSetError,
                    })),

                    _ => None,
                }
            })
            .collect::<QueryParserResult<Vec<_>>>()
            .and_then(|defaults| {
                // Checks all fields on the provided input object. This will catch extra / unknown fields and parsing errors.
                object
                    .into_iter()
                    .map(|(k, v)| match schema_object.find_field(k.as_str()) {
                        Some(field) => Self::parse_input_value(path.add(field.name.clone()), v, &field.field_types)
                            .map(|parsed| (k, parsed)),

                        None => Err(QueryParserError {
                            path: path.add(k),
                            error_kind: QueryParserErrorKind::FieldNotFoundError,
                        }),
                    })
                    .collect::<QueryParserResult<Vec<_>>>()
                    .map(|mut tuples| {
                        tuples.extend(defaults.into_iter());
                        tuples.into_iter().collect()
                    })
            })
            .and_then(|map: ParsedInputMap| {
                let num_fields = map.len();
                let too_many = schema_object
                    .constraints
                    .max_num_fields
                    .map(|max| num_fields > max)
                    .unwrap_or(false);

                let too_few = schema_object
                    .constraints
                    .min_num_fields
                    .map(|min| num_fields < min)
                    .unwrap_or(false);

                if too_many || too_few {
                    Err(QueryParserError {
                        path: path,
                        error_kind: QueryParserErrorKind::FieldCountError(FieldCountError::new(
                            schema_object.constraints.min_num_fields.clone(),
                            schema_object.constraints.max_num_fields.clone(),
                            map.len(),
                        )),
                    })
                } else {
                    Ok(map)
                }
            })
    }
}

#[derive(Debug)]
struct Diff<'a, T: std::cmp::Eq + std::hash::Hash> {
    pub left: Vec<&'a T>,
    pub right: Vec<&'a T>,
    pub equal: Vec<&'a T>,
}

impl<'a, T: std::cmp::Eq + std::hash::Hash> Diff<'a, T> {
    fn new(left_side: &'a HashSet<T>, right_side: &'a HashSet<T>) -> Diff<'a, T> {
        let left: Vec<&T> = left_side.difference(right_side).collect();
        let right: Vec<&T> = right_side.difference(left_side).collect();
        let equal: Vec<&T> = left_side.intersection(right_side).collect();

        Diff { left, right, equal }
    }
}
