use crate::{TestError, TestResult};
use indexmap::IndexMap;
use itertools::Itertools;
use query_core::{
    constants::custom_types,
    schema::{
        constants::{self, json_null},
        InputField, InputObjectType, InputType, OutputField, QuerySchema,
    },
    ArgumentValue, ArgumentValueObject, Selection,
};
use query_structure::PrismaValue;
use request_handlers::{Action, FieldQuery, GraphQLProtocolAdapter, JsonSingleQuery, SelectionSet, SelectionSetValue};
use serde_json::{json, Value as JsonValue};
use user_facing_errors::query_engine::validation::ValidationError;

pub struct JsonRequest;

impl JsonRequest {
    /// Translates a GraphQL query to a JSON query. This is used to keep the same test-suite running on both protocols.
    pub fn from_graphql(gql: &str, query_schema: &QuerySchema) -> TestResult<JsonSingleQuery> {
        match GraphQLProtocolAdapter::convert_query_to_operation(gql, None) {
            Ok(operation) => {
                let operation_name = operation.name();
                let schema_field = query_schema
                    .find_query_field(operation_name)
                    .or_else(|| query_schema.find_mutation_field(operation_name))
                    .ok_or_else(|| ValidationError::unknown_argument(vec![], vec![operation_name], vec![]))?;

                let model_name = schema_field
                    .model()
                    .map(|m| query_schema.internal_data_model.walk(m).name().to_owned());
                let query_tag = schema_field.query_tag().unwrap().to_owned();
                let selection = operation.into_selection();

                let output = JsonSingleQuery {
                    model_name,
                    action: Action::new(query_tag),
                    query: graphql_selection_to_json_field_query(selection, &schema_field),
                };

                Ok(output)
            }
            Err(err) => Err(TestError::RequestHandlerError(err)),
        }
    }
}

fn graphql_selection_to_json_field_query(mut selection: Selection, schema_field: &OutputField<'_>) -> FieldQuery {
    FieldQuery {
        arguments: graphql_args_to_json_args(&mut selection, schema_field.arguments()),
        selection: graphql_selection_to_json_selection(selection, schema_field),
    }
}

fn graphql_args_to_json_args(
    selection: &mut Selection,
    args_fields: &[InputField<'_>],
) -> Option<IndexMap<String, JsonValue>> {
    if selection.arguments().is_empty() {
        return None;
    }

    let mut args: IndexMap<String, JsonValue> = IndexMap::new();

    for (arg_name, arg_value) in selection.arguments().iter().cloned() {
        let arg_field = args_fields.iter().find(|arg_field| arg_field.name == arg_name);
        let inferrer = FieldTypeInferrer::from_field(arg_field).infer(&arg_value);
        let json = arg_value_to_json(arg_value, inferrer);
        args.insert(arg_name, json);
    }

    Some(args)
}

fn arg_value_to_json(value: ArgumentValue, typ: InferredType) -> JsonValue {
    match (value, typ) {
        (ArgumentValue::Object(obj), InferredType::Object(typ)) => JsonValue::Object(
            obj.into_iter()
                .map(|(k, v)| {
                    let field = typ.find_field(&k);
                    let inferrer = FieldTypeInferrer::from_field(field);
                    let inferred_type = inferrer.infer(&v);

                    (k, arg_value_to_json(v, inferred_type))
                })
                .collect(),
        ),
        (obj @ ArgumentValue::Object(_), InferredType::FieldRef) => {
            let val = arg_value_to_json(obj, InferredType::Unknown);

            make_json_custom_type(custom_types::FIELD_REF, val)
        }
        (ArgumentValue::Object(obj), InferredType::Unknown) => {
            let obj = obj
                .into_iter()
                .map(|(k, v)| (k, arg_value_to_json(v, InferredType::Unknown)))
                .collect();

            JsonValue::Object(obj)
        }
        (ArgumentValue::Scalar(PrismaValue::Enum(str)), InferredType::JsonNullEnum) => {
            make_json_custom_type(custom_types::ENUM, JsonValue::String(str))
        }
        (ArgumentValue::Scalar(PrismaValue::String(str)), InferredType::Json) => {
            serde_json::from_str(&str).unwrap_or_else(|_| panic!("Expected {str} to be JSON."))
        }
        (ArgumentValue::Scalar(pv), InferredType::Unknown) => serde_json::to_value(pv).unwrap(),

        (ArgumentValue::List(list), InferredType::List(typ)) => {
            let values = list
                .into_iter()
                .map(|val| {
                    let inferred_typ = FieldTypeInferrer::new(Some(&[typ.clone()])).infer(&val);

                    arg_value_to_json(val, inferred_typ)
                })
                .collect_vec();

            JsonValue::Array(values)
        }
        (ArgumentValue::List(list), InferredType::Unknown) => JsonValue::Array(
            list.into_iter()
                .map(|val| arg_value_to_json(val, InferredType::Unknown))
                .collect_vec(),
        ),
        _ => unreachable!(),
    }
}

fn graphql_selection_to_json_selection(selection: Selection, schema_field: &OutputField<'_>) -> SelectionSet {
    let mut res: IndexMap<String, SelectionSetValue> = IndexMap::new();

    for nested_selection in selection.nested_selections().iter().cloned() {
        let no_args = nested_selection.arguments().is_empty();
        let no_nested_selection = nested_selection.nested_selections().is_empty();
        let selection_name = nested_selection.name().to_owned();
        let can_have_nested_selection = schema_field
            .field_type()
            .as_object_type()
            .and_then(|object| object.find_field(&selection_name))
            .filter(|field| field.field_type().as_object_type().is_some())
            .is_some();

        if no_args && no_nested_selection {
            if can_have_nested_selection {
                res.insert(
                    selection_name,
                    SelectionSetValue::Nested(FieldQuery {
                        arguments: None,
                        selection: SelectionSet::new(IndexMap::new()),
                    }),
                );
            } else {
                res.insert(selection_name, SelectionSetValue::Shorthand(true));
            }
        } else {
            let nested_field = schema_field
                .field_type()
                .as_object_type()
                .unwrap()
                .find_field(&selection_name)
                .unwrap();

            let nested =
                SelectionSetValue::Nested(graphql_selection_to_json_field_query(nested_selection, nested_field));

            res.insert(selection_name, nested);
        }
    }

    SelectionSet::new(res)
}

pub fn make_json_custom_type(typ: &str, val: JsonValue) -> JsonValue {
    json!({ custom_types::TYPE: typ, custom_types::VALUE: val })
}

/// Tiny abstraction which helps inferring what type a value should be coerced to
/// when translated from GraphQL to JSON.
struct FieldTypeInferrer<'a, 'b> {
    /// The list of input types of an input field.
    types: Option<&'b [InputType<'a>]>,
}

impl<'a, 'b> FieldTypeInferrer<'a, 'b> {
    pub(crate) fn new(types: Option<&'b [InputType<'a>]>) -> Self {
        Self { types }
    }

    pub(crate) fn from_field(field: Option<&'b InputField<'a>>) -> Self {
        Self {
            types: field.map(|field| field.field_types()),
        }
    }

    /// Given a list of input types and an ArgumentValue, attempts to infer to the best of our ability
    /// what types it should be transformed to.
    /// If we cannot confidently infer the type, we return InferredType::Unknown.
    pub(crate) fn infer(&self, value: &ArgumentValue) -> InferredType<'a> {
        if self.types.is_none() {
            return InferredType::Unknown;
        }

        match value {
            ArgumentValue::Object(obj) => {
                let is_field_ref_obj = obj.contains_key(constants::filters::UNDERSCORE_REF)
                    && obj.contains_key(constants::filters::UNDERSCORE_CONTAINER)
                    && obj.len() == 2;
                let schema_objects = self.get_object_types();

                match schema_objects {
                    Some(schema_objects) => match Self::obj_val_fits_obj_types(obj, &schema_objects) {
                        Some(_) if is_field_ref_obj => InferredType::FieldRef,
                        Some(typ) => InferredType::Object(typ),
                        None => InferredType::Unknown,
                    },
                    _ => InferredType::Unknown,
                }
            }
            ArgumentValue::Scalar(pv) => match pv {
                PrismaValue::String(_) if self.has_json() => InferredType::Json,
                PrismaValue::Enum(str) if Self::is_json_null_enum(str) => InferredType::JsonNullEnum,
                _ => InferredType::Unknown,
            },
            ArgumentValue::List(_) => {
                let list_type = self.get_list_type();

                match list_type {
                    Some(typ) => InferredType::List(typ),
                    None => InferredType::Unknown,
                }
            }
            ArgumentValue::FieldRef(_) | ArgumentValue::Raw(_) => unreachable!(),
        }
    }

    fn has_json(&self) -> bool {
        self.types
            .map(|types| types.iter().any(|typ| typ.is_json()))
            .unwrap_or(false)
    }

    fn get_object_types(&self) -> Option<Vec<InputObjectType<'a>>> {
        self.types
            .map(|types| types.iter().cloned().filter_map(|typ| typ.into_object()).collect_vec())
    }

    fn get_list_type(&self) -> Option<InputType<'a>> {
        self.types
            .and_then(|types| types.iter().find_map(|typ| typ.as_list()))
            .map(|typ| typ.to_owned())
    }

    fn is_json_null_enum(val: &str) -> bool {
        [json_null::DB_NULL, json_null::JSON_NULL, json_null::ANY_NULL].contains(&val)
    }

    fn obj_val_fits_obj_types(
        val: &ArgumentValueObject,
        schema_objects: &[InputObjectType<'a>],
    ) -> Option<InputObjectType<'a>> {
        schema_objects
            .iter()
            .find(|schema_object| val.keys().all(|key| schema_object.find_field(key).is_some()))
            .cloned()
    }
}

#[derive(Debug)]
enum InferredType<'a> {
    Object(InputObjectType<'a>),
    List(InputType<'a>),
    Json,
    JsonNullEnum,
    FieldRef,
    Unknown,
}
