//! Json serialisation for query engine IR

use crate::PrismaResult;
use indexmap::IndexMap;
use prisma_models::{GraphqlId, PrismaValue};
use query_core::response_ir::{Item, Response};
use rust_decimal::prelude::ToPrimitive;
use serde_json::{Map, Number, Value};
use std::sync::Arc;

pub fn serialize(responses: Vec<Response>) -> Value {
    let mut outer_envelope = Map::new();
    let mut data_envelope = Map::new();
    let mut errors: Vec<Value> = Vec::new();

    for response in responses {
        match response {
            Response::Data(name, item) => {
                data_envelope.insert(name, serialize_item(item));
            }
            Response::Error(err) => {
                let mut error_map = Map::new();
                error_map.insert("error".into(), Value::String(err.clone()));
                errors.push(Value::Object(error_map));
            }
        }
    }

    if !errors.is_empty() {
        outer_envelope.insert("errors".into(), Value::Array(errors));
    }

    outer_envelope.insert("data".into(), Value::Object(data_envelope));

    Value::Object(outer_envelope)
}

/// Recursively serialize query results
fn serialize_item(item: Item) -> Value {
    match item {
        Item::List(l) => Value::Array(serialize_list(l)),
        Item::Map(m) => Value::Object(serialize_map(m)),
        Item::Value(v) => serialize_prisma_value(v).unwrap(),

        // Todo we're serializing multiple times here.
        Item::Ref(r) => match Arc::try_unwrap(r) {
            Ok(inner) => serialize_item(inner),
            Err(r) => serialize_item((*r).clone()),
        },
    }
}

fn serialize_map(map: IndexMap<String, Item>) -> Map<String, Value> {
    map.into_iter().fold(Map::new(), |mut map, (k, v)| {
        map.insert(k, serialize_item(v));
        map
    })
}

fn serialize_list(list: Vec<Item>) -> Vec<Value> {
    list.into_iter().fold(Vec::new(), |mut vec, i| {
        vec.push(serialize_item(i));
        vec
    })
}

fn serialize_prisma_value(value: PrismaValue) -> PrismaResult<Value> {
    Ok(match value {
        PrismaValue::String(x) => Value::String(x),
        PrismaValue::Float(x) => serde_json::to_value(x.to_f64().expect("Decimal is not a f64."))
            .expect("Unable to serialize Decimal to JSON."),
        PrismaValue::Boolean(x) => Value::Bool(x),
        PrismaValue::DateTime(date) => Value::String(format!("{}", date.format("%Y-%m-%dT%H:%M:%S%.3fZ"))),
        PrismaValue::Enum(x) => Value::String(x.as_string()),
        PrismaValue::Int(x) => Value::Number(Number::from(x)),
        PrismaValue::Null => Value::Null,
        PrismaValue::Uuid(x) => Value::String(x.to_hyphenated().to_string()),
        PrismaValue::GraphqlId(x) => serialize_graphql_id(x)?,
        PrismaValue::List(_) => unreachable!(),
    })
}

fn serialize_graphql_id(id: GraphqlId) -> PrismaResult<Value> {
    Ok(match id {
        GraphqlId::String(x) => Value::String(x),
        GraphqlId::Int(x) => Value::Number(Number::from(x)),
        GraphqlId::UUID(x) => Value::String(x.to_hyphenated().to_string()),
    })
}
