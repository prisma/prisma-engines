use super::*;
use crate::schema::*;
use bigdecimal::{BigDecimal, ToPrimitive};
use chrono::prelude::*;
use indexmap::IndexMap;
use prisma_value::PrismaValue;
use psl::{datamodel_connector::IntType, dml::ValueGeneratorFn};
use std::{borrow::Borrow, collections::HashSet, convert::TryFrom, str::FromStr, sync::Arc};
use uuid::Uuid;

// todo: validate is one of!

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
        selections: &[Selection],
        schema_object: &ObjectTypeStrongRef,
    ) -> QueryParserResult<ParsedObject> {
        let path = parent_path.add(schema_object.identifier.name().to_owned());

        if selections.is_empty() {
            return Err(QueryParserError {
                path,
                error_kind: QueryParserErrorKind::FieldCountError(FieldCountError::new(Some(1), None, 0)),
            });
        }

        selections
            .iter()
            .map(|selection| match schema_object.find_field(selection.name()) {
                Some(ref field) => self.parse_field(path.clone(), selection, field),
                None => Err(QueryParserError {
                    path: path.add(selection.name().into()),
                    error_kind: QueryParserErrorKind::FieldNotFoundError,
                }),
            })
            .collect::<QueryParserResult<Vec<_>>>()
            .map(|fields| ParsedObject { fields })
    }

    /// Parses and validates a selection against a schema (output) field.
    fn parse_field(
        &self,
        parent_path: QueryPath,
        selection: &Selection,
        schema_field: &OutputFieldRef,
    ) -> QueryParserResult<FieldPair> {
        let path = parent_path.add(schema_field.name.clone());

        // Parse and validate all provided arguments for the field
        self.parse_arguments(path.clone(), schema_field, selection.arguments())
            .and_then(|arguments| {
                // If the output type of the field is an object type of any form, validate the sub selection as well.
                let nested_fields = schema_field
                    .field_type
                    .as_object_type()
                    .map(|obj| self.parse_object(path.clone(), selection.nested_selections(), &obj));

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
            })
    }

    /// Parses and validates selection arguments against a schema defined field.
    fn parse_arguments(
        &self,
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
                    Some((_, value)) => Some(self.parse_input_value(path, value, &schema_input_arg.field_types).map(
                        |value| ParsedArgument {
                            name: schema_input_arg.name.clone(),
                            value,
                        },
                    )),

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
    fn parse_input_value(
        &self,
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
                (_, InputType::Scalar(scalar)) => self
                    .parse_scalar(&parent_path, value, &scalar)
                    .map(ParsedInputValue::Single),

                // Enum handling
                (QueryValue::Enum(_), InputType::Enum(et)) => self.parse_enum(&parent_path, value, &et.into_arc()),
                (QueryValue::String(_), InputType::Enum(et)) => self.parse_enum(&parent_path, value, &et.into_arc()),
                (QueryValue::Boolean(_), InputType::Enum(et)) => self.parse_enum(&parent_path, value, &et.into_arc()),

                // List handling.
                (QueryValue::List(values), InputType::List(l)) => self
                    .parse_list(&parent_path, values.clone(), &l)
                    .map(ParsedInputValue::List),

                // Object handling
                (QueryValue::Object(o), InputType::Object(obj)) => self
                    .parse_input_object(parent_path.clone(), o.clone(), obj.into_arc())
                    .map(ParsedInputValue::Map),

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
    fn parse_scalar(
        &self,
        parent_path: &QueryPath,
        value: QueryValue,
        scalar_type: &ScalarType,
    ) -> QueryParserResult<PrismaValue> {
        match (value, scalar_type.clone()) {
            (QueryValue::String(s), ScalarType::String) => Ok(PrismaValue::String(s)),
            (QueryValue::String(s), ScalarType::Xml) => Ok(PrismaValue::Xml(s)),
            (QueryValue::String(s), ScalarType::JsonList) => self.parse_json_list(parent_path, &s),
            (QueryValue::String(s), ScalarType::Bytes) => self.parse_bytes(parent_path, s),
            (QueryValue::String(s), ScalarType::Decimal) => self.parse_decimal(parent_path, s),
            (QueryValue::String(s), ScalarType::BigInt) => self.parse_bigint(parent_path, s),
            (QueryValue::String(s), ScalarType::UUID) => {
                self.parse_uuid(parent_path, s.as_str()).map(PrismaValue::Uuid)
            }
            (QueryValue::String(s), ScalarType::Json) => {
                Ok(PrismaValue::Json(self.parse_json(parent_path, &s).map(|_| s)?))
            }
            (QueryValue::String(s), ScalarType::DateTime) => {
                self.parse_datetime(parent_path, s.as_str()).map(PrismaValue::DateTime)
            }
            (QueryValue::DateTime(s), ScalarType::DateTime) => Ok(PrismaValue::DateTime(s)),

            (QueryValue::Int(i), ScalarType::Int(int_type)) => {
                let fits_in_specified_type = match &int_type {
                    IntType::Signed8 => i8::try_from(i).is_ok(),
                    IntType::Signed16 => i16::try_from(i).is_ok(),
                    IntType::Signed24 => fits_in_i24(i),
                    IntType::Signed32 => i32::try_from(i).is_ok(),
                    IntType::Unsigned8 => u8::try_from(i).is_ok(),
                    IntType::Unsigned16 => u16::try_from(i).is_ok(),
                    IntType::Unsigned24 => fits_in_u24(i),
                    IntType::Unsigned32 => u32::try_from(i).is_ok(),
                };

                if fits_in_specified_type {
                    Ok(PrismaValue::Int(i))
                } else {
                    Err(QueryParserError::new(parent_path.clone(), QueryParserErrorKind::ValueFitError(
                        format!("Unable to fit integer value '{}' into a {} for field '{}'. If you're trying to store large integers, consider using `BigInt`.", i, int_type, parent_path.last().unwrap()))))
                }
            },
            (QueryValue::Int(i), ScalarType::Float) => Ok(PrismaValue::Float(BigDecimal::from(i))),
            (QueryValue::Int(i), ScalarType::Decimal) => Ok(PrismaValue::Float(BigDecimal::from(i))),
            (QueryValue::Int(i), ScalarType::BigInt) => Ok(PrismaValue::BigInt(i)),

            (QueryValue::Float(f), ScalarType::Float) => Ok(PrismaValue::Float(f)),
            (QueryValue::Float(f), ScalarType::Decimal) => Ok(PrismaValue::Float(f)),
            (QueryValue::Float(f), ScalarType::Int(_)) => match f.to_i64() {
                Some(converted) => Ok(PrismaValue::Int(converted)),
                None => Err(QueryParserError::new(parent_path.clone(), QueryParserErrorKind::ValueFitError(
                            format!("Unable to fit float value (or large JS integer serialized in exponent notation) '{}' into a 64 Bit signed integer for field '{}'. If you're trying to store large integers, consider using `BigInt`.", f, parent_path.last().unwrap())))),
            },

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

    fn parse_datetime(&self, path: &QueryPath, s: &str) -> QueryParserResult<DateTime<FixedOffset>> {
        DateTime::parse_from_rfc3339(s).map_err(|err| QueryParserError {
            path: path.clone(),
            error_kind: QueryParserErrorKind::ValueParseError(format!(
                "Invalid DateTime: '{}' (must be ISO 8601 compatible). Underlying error: {}",
                s, err
            )),
        })
    }

    fn parse_bytes(&self, path: &QueryPath, s: String) -> QueryParserResult<PrismaValue> {
        prisma_value::decode_bytes(&s)
            .map(PrismaValue::Bytes)
            .map_err(|_| QueryParserError {
                path: path.clone(),
                error_kind: QueryParserErrorKind::ValueParseError(format!(
                    "'{}' is not a valid base64 encoded string.",
                    s
                )),
            })
    }

    fn parse_decimal(&self, path: &QueryPath, s: String) -> QueryParserResult<PrismaValue> {
        BigDecimal::from_str(&s)
            .map(PrismaValue::Float)
            .map_err(|_| QueryParserError {
                path: path.clone(),
                error_kind: QueryParserErrorKind::ValueParseError(format!("'{}' is not a valid decimal string", s)),
            })
    }

    fn parse_bigint(&self, path: &QueryPath, s: String) -> QueryParserResult<PrismaValue> {
        s.parse::<i64>().map(PrismaValue::BigInt).map_err(|_| QueryParserError {
            path: path.clone(),
            error_kind: QueryParserErrorKind::ValueParseError(format!("'{}' is not a valid big integer string", s)),
        })
    }

    // [DTODO] This is likely incorrect or at least using the wrong abstractions.
    fn parse_json_list(&self, path: &QueryPath, s: &str) -> QueryParserResult<PrismaValue> {
        let json = self.parse_json(path, s)?;

        let values = json.as_array().ok_or_else(|| QueryParserError {
            path: path.clone(),
            error_kind: QueryParserErrorKind::AssertionError("JSON parameter needs to be an array".into()),
        })?;

        let mut prisma_values = Vec::with_capacity(values.len());

        for v in values.iter() {
            let pv = PrismaValue::try_from(v.clone()).map_err(|_| QueryParserError {
                path: path.clone(),
                error_kind: QueryParserErrorKind::AssertionError("Nested JSON arguments are not supported".into()),
            })?;

            prisma_values.push(pv);
        }

        Ok(PrismaValue::List(prisma_values))
    }

    fn parse_json(&self, path: &QueryPath, s: &str) -> QueryParserResult<serde_json::Value> {
        serde_json::from_str(s).map_err(|err| QueryParserError {
            path: path.clone(),
            error_kind: QueryParserErrorKind::ValueParseError(format!("Invalid json: {}", err)),
        })
    }

    fn parse_uuid(&self, path: &QueryPath, s: &str) -> QueryParserResult<Uuid> {
        Uuid::parse_str(s).map_err(|err| QueryParserError {
            path: path.clone(),
            error_kind: QueryParserErrorKind::ValueParseError(format!("Invalid UUID: {}", err)),
        })
    }

    fn parse_list(
        &self,
        path: &QueryPath,
        values: Vec<QueryValue>,
        value_type: &InputType,
    ) -> QueryParserResult<Vec<ParsedInputValue>> {
        values
            .into_iter()
            .map(|val| self.parse_input_value(path.clone(), val, &[value_type.clone()]))
            .collect::<QueryParserResult<Vec<ParsedInputValue>>>()
    }

    fn parse_enum(&self, path: &QueryPath, val: QueryValue, typ: &EnumTypeRef) -> QueryParserResult<ParsedInputValue> {
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
        object: IndexMap<String, QueryValue>,
        schema_object: InputObjectTypeStrongRef,
    ) -> QueryParserResult<ParsedInputMap> {
        let path = parent_path.add(schema_object.identifier.name().to_owned());
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
        let defaults = diff
            .left
            .into_iter()
            .filter_map(|unset_field_name| {
                let field = schema_object.find_field(*unset_field_name).unwrap();
                let path = path.add(field.name.clone());

                // If the input field has a default, add the default to the result.
                // If it's not optional and has no default, a required field has not been provided.
                match &field.default_value {
                    Some(default_value) => {
                        let query_value = match &default_value.kind {
                            psl::dml::DefaultKind::Expression(ref expr)
                                if matches!(expr.generator(), ValueGeneratorFn::Now) =>
                            {
                                self.default_now.clone().into()
                            }
                            _ => default_value.get()?.into(),
                        };

                        match self.parse_input_value(path, query_value, &field.field_types) {
                            Ok(value) => Some(Ok((field.name.clone(), value))),
                            Err(err) => Some(Err(err)),
                        }
                    }
                    None => {
                        if field.is_required {
                            let kind = QueryParserErrorKind::RequiredValueNotSetError;
                            Some(Err(QueryParserError::new(path, kind)))
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
            .map(|(k, v)| {
                let field = schema_object.find_field(k.as_str()).ok_or_else(|| {
                    let kind = QueryParserErrorKind::FieldNotFoundError;
                    QueryParserError::new(path.add(k.clone()), kind)
                })?;

                let path = path.add(field.name.clone());
                let parsed = self.parse_input_value(path, v, &field.field_types)?;

                Ok((k, parsed))
            })
            .collect::<QueryParserResult<ParsedInputMap>>()?;

        map.extend(defaults.into_iter());

        // Ensure the constraints are upheld.
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
            let error_kind = QueryParserErrorKind::FieldCountError(FieldCountError::new(
                schema_object.constraints.min_num_fields,
                schema_object.constraints.max_num_fields,
                map.len(),
            ));
            return Err(QueryParserError::new(path, error_kind));
        }

        map.set_tag(schema_object.tag.clone());

        Ok(map)
    }
}

#[derive(Debug)]
struct Diff<'a, T: std::cmp::Eq + std::hash::Hash> {
    pub left: Vec<&'a T>,
    pub right: Vec<&'a T>,
    pub _equal: Vec<&'a T>,
}

impl<'a, T: std::cmp::Eq + std::hash::Hash> Diff<'a, T> {
    fn new(left_side: &'a HashSet<T>, right_side: &'a HashSet<T>) -> Diff<'a, T> {
        let left: Vec<&T> = left_side.difference(right_side).collect();
        let right: Vec<&T> = right_side.difference(left_side).collect();
        let _equal: Vec<&T> = left_side.intersection(right_side).collect();

        Diff { left, right, _equal }
    }
}

fn fits_in_i24(from: i64) -> bool {
    let i24_min = -8388608;
    let i24_max = 8388607;

    from >= i24_min && from <= i24_max
}

fn fits_in_u24(from: i64) -> bool {
    let u24_min = 0;
    let u24_max = 16777215;

    from >= u24_min && from <= u24_max
}
