use indexmap::IndexMap;
use prisma_models::PrismaValue;
use query_core::{constants::custom_types, schema::QuerySchemaRef, Selection};
use request_handlers::{Action, FieldQuery, GraphQLProtocolAdapter, JsonSingleQuery, SelectionSet, SelectionSetValue};
use serde_json::{json, Value as JsonValue};

use crate::TestResult;

pub struct JsonRequest;

impl JsonRequest {
    pub fn from_graphql(gql: &str, query_schema: &QuerySchemaRef) -> TestResult<JsonSingleQuery> {
        let operation = GraphQLProtocolAdapter::convert_query_to_operation(gql, None).unwrap();
        let operation_name = operation.name();
        let schema_field = query_schema
            .find_query_field(operation_name)
            .unwrap_or_else(|| query_schema.find_mutation_field(operation_name).unwrap());
        let model_name = schema_field.model().as_ref().map(|m| m.name().to_owned());
        let query_tag = schema_field.query_tag().unwrap().to_owned();
        let mut selection = operation.into_selection();

        let output = JsonSingleQuery {
            model_name,
            action: Action::new(query_tag),
            query: graphql_selection_to_json_field_query(&mut selection),
        };

        Ok(output)
    }
}

fn graphql_selection_to_json_field_query(selection: &mut Selection) -> FieldQuery {
    FieldQuery {
        arguments: graphql_args_to_json_args(selection),
        selection: graphql_selection_to_json_selection(selection),
    }
}

fn graphql_args_to_json_args(selection: &mut Selection) -> Option<IndexMap<String, JsonValue>> {
    if selection.arguments().is_empty() {
        return None;
    }

    let mut args: IndexMap<String, JsonValue> = IndexMap::new();

    while let Some((arg_name, arg_value)) = selection.pop_argument() {
        args.insert(arg_name, prisma_value_to_json(arg_value));
    }

    Some(args)
}

fn prisma_value_to_json(pv: PrismaValue) -> JsonValue {
    match pv {
        PrismaValue::Object(obj) => {
            let is_field_ref_obj = obj.iter().any(|(k, _)| k == "_ref");
            let map: serde_json::Map<String, JsonValue> =
                obj.into_iter().map(|(k, v)| (k, prisma_value_to_json(v))).collect();

            if is_field_ref_obj {
                json!({ custom_types::TYPE: custom_types::FIELD_REF, custom_types::VALUE: JsonValue::Object(map) })
            } else {
                JsonValue::Object(map)
            }
        }
        PrismaValue::List(list) => JsonValue::Array(list.into_iter().map(prisma_value_to_json).collect()),
        _ => serde_json::to_value(&pv).unwrap(),
    }
}

fn graphql_selection_to_json_selection(selection: &mut Selection) -> SelectionSet {
    let mut res: IndexMap<String, SelectionSetValue> = IndexMap::new();

    for mut nested_selection in selection.nested_selections().iter().cloned() {
        let no_args = nested_selection.arguments().is_empty();
        let no_nested_selection = nested_selection.nested_selections().is_empty();
        let selection_name = nested_selection.name().to_owned();

        if no_args && no_nested_selection {
            res.insert(selection_name, SelectionSetValue::Shorthand(true));
        } else {
            let nested = SelectionSetValue::Nested(graphql_selection_to_json_field_query(&mut nested_selection));

            res.insert(selection_name, nested);
        }
    }

    SelectionSet::new(res)
}
