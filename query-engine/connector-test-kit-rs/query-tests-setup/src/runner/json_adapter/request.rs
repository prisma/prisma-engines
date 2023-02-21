use indexmap::IndexMap;
use itertools::Itertools;
use prisma_models::PrismaValue;
use query_core::{
    constants::custom_types,
    schema::{InputFieldRef, InputObjectTypeStrongRef, InputType, OutputFieldRef, QuerySchemaRef},
    schema_builder::constants::{self, json_null},
    ArgumentValue, ArgumentValueObject, Selection,
};
use request_handlers::{Action, FieldQuery, GraphQLProtocolAdapter, JsonSingleQuery, SelectionSet, SelectionSetValue};
use serde_json::{json, Value as JsonValue};

use crate::TestResult;

pub struct JsonRequest;

impl JsonRequest {
    /// Translates a GraphQL query to a JSON query. This is used to keep the same test-suite running on both protocols.
    pub fn from_graphql(gql: &str, query_schema: &QuerySchemaRef) -> TestResult<JsonSingleQuery> {
        let operation = GraphQLProtocolAdapter::convert_query_to_operation(gql, None).unwrap();
        let operation_name = operation.name();
        let schema_field = query_schema
            .find_query_field(operation_name)
            .unwrap_or_else(|| query_schema.find_mutation_field(operation_name).unwrap());
        let model_name = schema_field.model().as_ref().map(|m| m.name().to_owned());
        let query_tag = schema_field.query_tag().unwrap().to_owned();
        let selection = operation.into_selection();

        let output = JsonSingleQuery {
            model_name,
            action: Action::new(query_tag),
            query: graphql_selection_to_json_field_query(selection, &schema_field),
        };

        Ok(output)
    }
}

fn graphql_selection_to_json_field_query(mut selection: Selection, schema_field: &OutputFieldRef) -> FieldQuery {
    FieldQuery {
        arguments: graphql_args_to_json_args(&mut selection, &schema_field.arguments),
        selection: graphql_selection_to_json_selection(selection, schema_field),
    }
}

fn graphql_args_to_json_args(
    selection: &mut Selection,
    args_fields: &[InputFieldRef],
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
                    let inferrer = FieldTypeInferrer::from_field(field.as_ref());
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
                    let inferred_typ = FieldTypeInferrer::new(Some(&vec![typ.clone()])).infer(&val);

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

fn graphql_selection_to_json_selection(selection: Selection, schema_field: &OutputFieldRef) -> SelectionSet {
    let mut res: IndexMap<String, SelectionSetValue> = IndexMap::new();

    for nested_selection in selection.nested_selections().iter().cloned() {
        let no_args = nested_selection.arguments().is_empty();
        let no_nested_selection = nested_selection.nested_selections().is_empty();
        let selection_name = nested_selection.name().to_owned();

        if no_args && no_nested_selection {
            res.insert(selection_name, SelectionSetValue::Shorthand(true));
        } else {
            let nested_field = schema_field
                .field_type
                .as_object_type()
                .unwrap()
                .find_field(&selection_name)
                .unwrap();

            let nested =
                SelectionSetValue::Nested(graphql_selection_to_json_field_query(nested_selection, &nested_field));

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
struct FieldTypeInferrer<'a> {
    /// The list of input types of an input field.
    types: Option<&'a Vec<InputType>>,
}

impl<'a> FieldTypeInferrer<'a> {
    pub(crate) fn new(types: Option<&'a Vec<InputType>>) -> Self {
        Self { types }
    }

    pub(crate) fn from_field(field: Option<&'a InputFieldRef>) -> Self {
        Self {
            types: field.map(|field| &field.field_types),
        }
    }

    /// Given a list of input types and an ArgumentValue, attempts to infer to the best of our ability
    /// what types it should be transformed to.
    /// If we cannot confidently infer the type, we return InferredType::Unknown.
    pub(crate) fn infer(&self, value: &ArgumentValue) -> InferredType {
        if self.types.is_none() {
            return InferredType::Unknown;
        }

        match value {
            ArgumentValue::Object(obj) => {
                let is_field_ref_obj = obj.contains_key(constants::filters::UNDERSCORE_REF) && obj.len() == 1;
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
            ArgumentValue::FieldRef(_) => unreachable!(),
        }
    }

    fn has_json(&self) -> bool {
        self.types
            .map(|types| types.iter().any(|typ| typ.is_json()))
            .unwrap_or(false)
    }

    fn get_object_types(&self) -> Option<Vec<InputObjectTypeStrongRef>> {
        self.types
            .map(|types| types.iter().filter_map(|typ| typ.as_object()).collect_vec())
    }

    fn get_list_type(&self) -> Option<InputType> {
        self.types
            .and_then(|types| types.iter().find_map(|typ| typ.as_list()))
            .map(|typ| typ.to_owned())
    }

    fn is_json_null_enum(val: &str) -> bool {
        [json_null::DB_NULL, json_null::JSON_NULL, json_null::ANY_NULL].contains(&val)
    }

    fn obj_val_fits_obj_types(
        val: &ArgumentValueObject,
        schema_objects: &[InputObjectTypeStrongRef],
    ) -> Option<InputObjectTypeStrongRef> {
        schema_objects
            .iter()
            .find(|schema_object| val.keys().all(|key| schema_object.find_field(key).is_some()))
            .cloned()
    }
}

#[derive(Debug)]
enum InferredType {
    Object(InputObjectTypeStrongRef),
    List(InputType),
    Json,
    JsonNullEnum,
    FieldRef,
    Unknown,
}
