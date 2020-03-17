use super::*;
use crate::schema::*;
use chrono::prelude::*;
use prisma_value::PrismaValue;
use rust_decimal::{prelude::ToPrimitive, Decimal};
use std::{
    borrow::Borrow,
    collections::{BTreeMap, HashSet},
    convert::TryFrom,
    sync::Arc,
};
use uuid::Uuid;

pub struct QueryDocumentParser;

// Todo:
// - Use error collections instead of letting first error win.
// - UUID ids are not encoded in any useful way in the schema.
// - Alias handling in query names.
impl QueryDocumentParser {
    /// Parses and validates a set of selections against a schema (output) object.
    /// On an output object, nullability designates whether or not an output field can be null
    /// (in contrast, nullability on an input object means whether or not a field as to be provided).
    /// The above is the reason we don't need to check nullability here, as it is done by the output
    /// validation in the serialization step.
    pub fn parse_object(
        selections: &[Selection],
        schema_object: &ObjectTypeStrongRef,
    ) -> QueryParserResult<ParsedObject> {
        if selections.is_empty() {
            return Err(QueryParserError::ObjectValidationError {
                object_name: schema_object.name().to_string(),
                inner: Box::new(QueryParserError::AtLeastOneSelectionError),
            });
        }

        selections
            .iter()
            .map(|selection| {
                let parsed_field = match schema_object.find_field(selection.name()) {
                    Some(ref field) => Self::parse_field(selection, field),
                    None => Err(QueryParserError::FieldValidationError {
                        field_name: selection.name().into(),
                        inner: Box::new(QueryParserError::FieldNotFoundError),
                    }),
                };

                parsed_field.map_err(|err| QueryParserError::ObjectValidationError {
                    object_name: schema_object.name().to_string(),
                    inner: Box::new(err),
                })
            })
            .collect::<QueryParserResult<Vec<ParsedField>>>()
            .map(|fields| ParsedObject { fields })
    }

    /// Parses and validates a selection against a schema (output) field.
    fn parse_field(
        selection: &Selection,
        schema_field: &FieldRef,
    ) -> QueryParserResult<ParsedField> {
        // Parse and validate all provided arguments for the field
        Self::parse_arguments(schema_field, selection.arguments())
            .and_then(|arguments| {
                // If the output type of the field is an object type of any form, validate the sub selection as well.
                let nested_fields = schema_field
                    .field_type
                    .as_object_type()
                    .map(|obj| Self::parse_object(selection.nested_selections(), &obj));

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
            .map_err(|err| QueryParserError::FieldValidationError {
                field_name: schema_field.name.clone(),
                inner: Box::new(err),
            })
    }

    /// Parses and validates selection arguments against a schema defined field.
    // Todo if needed at some point, argument default handling can be added here.
    pub fn parse_arguments(
        schema_field: &FieldRef,
        given_arguments: &[(String, QueryValue)],
    ) -> QueryParserResult<Vec<ParsedArgument>> {
        let left: HashSet<&str> = schema_field
            .arguments
            .iter()
            .map(|arg| arg.name.as_str())
            .collect();
        let right: HashSet<&str> = given_arguments.iter().map(|arg| arg.0.as_str()).collect();
        let diff = Diff::new(&left, &right);

        // All arguments that are not in the schema cause an error.
        diff.right
            .into_iter()
            .map(|extra_arg| {
                Err(QueryParserError::ArgumentValidationError {
                    argument: (*extra_arg).to_owned(),
                    inner: Box::new(QueryParserError::ArgumentNotFoundError),
                })
            })
            .collect::<QueryParserResult<Vec<()>>>()?;

        // Check remaining arguments
        schema_field
            .arguments
            .iter()
            .filter_map(|schema_arg| {
                // Match schema field to a field in the incoming document
                let selection_arg: Option<(String, QueryValue)> = given_arguments
                    .iter()
                    .find(|given_argument| given_argument.0 == schema_arg.name)
                    .cloned();

                // If the arg can be found, parse the provided query value into a list / object / PrismaValue.
                //
                // If the arg can _not_ be found, pretend the arg was provided with a Null.
                // Run the validation against the Null value to check if it needs to be provided, but disregard the result if it succeeded.
                let (selection_arg, retain) = match selection_arg {
                    Some(arg) => (arg, true),
                    None => ((schema_arg.name.clone(), QueryValue::Null), false),
                };

                let result = Self::parse_input_value(selection_arg.1, &schema_arg.argument_type)
                    .map(|value| ParsedArgument {
                        name: schema_arg.name.clone(),
                        value,
                    })
                    .map_err(|err| QueryParserError::ArgumentValidationError {
                        argument: schema_arg.name.clone(),
                        inner: Box::new(err),
                    });

                if result.is_err() || retain {
                    Some(result)
                } else {
                    None
                }
            })
            .collect::<Vec<QueryParserResult<ParsedArgument>>>()
            .into_iter()
            .collect()
    }

    /// Parses and validates a QueryValue against an InputType, recursively.
    #[rustfmt::skip]
    pub fn parse_input_value(value: QueryValue, input_type: &InputType) -> QueryParserResult<ParsedInputValue> {
        // todo figure out what is up with enums
        match (&value, input_type) {
            // Handle null inputs
            (QueryValue::Null, InputType::Opt(_))           => Ok(ParsedInputValue::Single(PrismaValue::Null)),
            (_, InputType::Opt(ref inner))                  => Self::parse_input_value(value, inner),

            // The optional handling above guarantees that if we hit a Null here, a required value is missing.
            (QueryValue::Null, _)                           => Err(QueryParserError::RequiredValueNotSetError),

            // Scalar and enum handling.
            (_, InputType::Scalar(scalar))                  => Self::parse_scalar(value, &scalar).map(ParsedInputValue::Single),
            (QueryValue::Enum(_), InputType::Enum(et))      => Self::parse_enum(value, et),
            (QueryValue::String(_), InputType::Enum(et))      => Self::parse_enum(value, et),
            (QueryValue::Boolean(_), InputType::Enum(et))      => Self::parse_enum(value, et),

            // List and object handling.
            (QueryValue::List(values), InputType::List(l))  => Self::parse_list(values.clone(), &l).map(ParsedInputValue::List),
            (_, InputType::List(l))                         => Self::parse_list(vec![value], &l).map(ParsedInputValue::List),
            (QueryValue::Object(o), InputType::Object(obj)) => Self::parse_input_object(o.clone(), obj.into_arc()).map(ParsedInputValue::Map),
            (_, input_type)                                 => Err(QueryParserError::ValueTypeMismatchError { have: value, want: input_type.clone() }),
        }
    }

    /// Attempts to parse given query value into a concrete PrismaValue based on given scalar type.
    #[rustfmt::skip]
    pub fn parse_scalar(value: QueryValue, scalar_type: &ScalarType) -> QueryParserResult<PrismaValue> {
        match (value, scalar_type.clone()) {
            (QueryValue::Null, _)                         => Ok(PrismaValue::Null),
            (QueryValue::String(s), ScalarType::String)   => Ok(PrismaValue::String(s)),
            (QueryValue::String(s), ScalarType::DateTime) => Self::parse_datetime(s.as_str()).map(PrismaValue::DateTime),
            (QueryValue::String(s), ScalarType::JsonList) => Self::parse_json_list(&s),
            (QueryValue::String(s), ScalarType::UUID)     => Self::parse_uuid(s.as_str()).map(PrismaValue::Uuid),
            (QueryValue::Int(i), ScalarType::Float)       => Ok(PrismaValue::Float(Decimal::from(i))),
            (QueryValue::Int(i), ScalarType::Int)         => Ok(PrismaValue::Int(i)),
            (QueryValue::Float(f), ScalarType::Float)     => Ok(PrismaValue::Float(f)),
            (QueryValue::Float(f), ScalarType::Int)       => {
                Ok(PrismaValue::Int(f.to_i64().unwrap()))
            },
            (QueryValue::Boolean(b), ScalarType::Boolean) => Ok(PrismaValue::Boolean(b)),

            // All other combinations are invalid.
            (qv, _)                                       => Err(QueryParserError::ValueTypeMismatchError { have: qv, want: InputType::Scalar(scalar_type.clone()) }),
        }
    }

    pub fn parse_datetime(s: &str) -> QueryParserResult<DateTime<Utc>> {
        let fmt = "%Y-%m-%dT%H:%M:%S%.3f";
        Utc.datetime_from_str(s.trim_end_matches('Z'), fmt)
            .map(|dt| DateTime::<Utc>::from_utc(dt.naive_utc(), Utc))
            .map_err(|err| {
                QueryParserError::ValueParseError(format!(
                    "Invalid DateTime: {} DateTime must adhere to format: %Y-%m-%dT%H:%M:%S%.3f",
                    err
                ))
            })
    }

    pub fn parse_json_list(s: &str) -> QueryParserResult<PrismaValue> {
        let json = Self::parse_json(s)?;

        let values = json.as_array().ok_or_else(|| {
            QueryParserError::AssertionError("JSON parameter needs to be an array".into())
        })?;

        let mut prisma_values = Vec::with_capacity(values.len());

        for v in values.into_iter() {
            let pv = PrismaValue::try_from(v.clone()).map_err(|_| {
                QueryParserError::AssertionError("Nested JSON arguments are not supported".into())
            })?;

            prisma_values.push(pv);
        }

        Ok(PrismaValue::List(prisma_values))
    }

    pub fn parse_json(s: &str) -> QueryParserResult<serde_json::Value> {
        serde_json::from_str(s)
            .map_err(|err| QueryParserError::ValueParseError(format!("Invalid json: {}", err)))
    }

    pub fn parse_uuid(s: &str) -> QueryParserResult<Uuid> {
        Uuid::parse_str(s)
            .map_err(|err| QueryParserError::ValueParseError(format!("Invalid UUID: {}", err)))
    }

    pub fn parse_list(
        values: Vec<QueryValue>,
        value_type: &InputType,
    ) -> QueryParserResult<Vec<ParsedInputValue>> {
        values
            .into_iter()
            .map(|val| Self::parse_input_value(val, value_type))
            .collect::<QueryParserResult<Vec<ParsedInputValue>>>()
    }

    pub fn parse_enum(val: QueryValue, typ: &EnumTypeRef) -> QueryParserResult<ParsedInputValue> {
        let raw = match val {
            QueryValue::Enum(s) => s,
            QueryValue::String(s) => s,
            QueryValue::Boolean(b) => if b { "true" } else { "false " }.to_owned(), // Case where a bool was misinterpreted as constant literal
            _ => {
                return Err(QueryParserError::ValueParseError(format!(
                    "Unexpected Enum value type {:?} for enum {}",
                    val,
                    typ.name()
                )));
            }
        };

        match typ.borrow() {
            EnumType::Internal(i) => match i.map_input_value(&raw) {
                Some(value) => Ok(ParsedInputValue::Single(value)),
                None => Err(QueryParserError::ValueParseError(format!(
                    "Enum value '{}' is invalid for enum type {}",
                    raw, i.name
                ))),
            },
            EnumType::OrderBy(ord) => match ord.value_for(raw.as_str()) {
                Some(val) => Ok(ParsedInputValue::OrderBy(val.clone())),
                None => Err(QueryParserError::ValueParseError(format!(
                    "Enum value '{}' is invalid for enum type {}",
                    raw, ord.name
                ))),
            },
        }
    }

    /// Parses and validates an input object recursively.
    pub fn parse_input_object(
        object: BTreeMap<String, QueryValue>,
        schema_object: InputObjectTypeStrongRef,
    ) -> QueryParserResult<ParsedInputMap> {
        let left: HashSet<&str> = schema_object
            .get_fields()
            .iter()
            .map(|field| field.name.as_str())
            .collect();

        let right: HashSet<&str> = object.keys().map(|k| k.as_str()).collect();
        let diff = Diff::new(&left, &right);

        // First, check that all fields not provided in the query (left diff) are optional,
        // i.e. run the validation but disregard the result, or have defaults, in which case the
        // value pair gets added to the result.
        diff.left
            .into_iter()
            .filter_map(|unset_field_name| {
                let field = schema_object.find_field(*unset_field_name).unwrap();
                let default_pair = field.default_value.clone().map(|def| (&field.name, def));

                match default_pair {
                    // If the input field has a default, add the default to the result.
                    Some((k, dv)) => {
                        match Self::parse_input_field(dv.get_as_prisma_value().into(), &field) {
                            Ok(value) => Some(Ok((k.clone(), value))),
                            Err(err) => Some(Err(err)),
                        }
                    }
                    // Finally, if nothing is found, parse the input value with Null but disregard the result,
                    // except errors, which are propagated.
                    None => match Self::parse_input_field(QueryValue::Null, &field) {
                        Ok(_) => None,
                        Err(err) => Some(Err(err)),
                    },
                }
            })
            .collect::<QueryParserResult<Vec<_>>>()
            .and_then(|defaults| {
                // Checks all fields on the provided input object. This will catch extra, unknown fields and parsing errors.
                object
                    .into_iter()
                    .map(|(k, v)| match schema_object.find_field(k.as_str()) {
                        Some(field) => Self::parse_input_field(v, &field).map(|parsed| (k, parsed)),

                        None => Err(QueryParserError::FieldValidationError {
                            field_name: k.clone(),
                            inner: Box::new(QueryParserError::FieldNotFoundError),
                        }),
                    })
                    .collect::<QueryParserResult<Vec<_>>>()
                    .map(|mut tuples| {
                        tuples.extend(defaults.into_iter());
                        tuples.into_iter().collect()
                    })
            })
            .map_err(|err| QueryParserError::ObjectValidationError {
                object_name: schema_object.name.clone(),
                inner: Box::new(err),
            })
    }

    /// Parses and validates an input query value against a schema input field.
    pub fn parse_input_field(
        value: QueryValue,
        schema_field: &InputFieldRef,
    ) -> QueryParserResult<ParsedInputValue> {
        Self::parse_input_value(value, &schema_field.field_type).map_err(|err| {
            QueryParserError::FieldValidationError {
                field_name: schema_field.name.clone(),
                inner: Box::new(err),
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
