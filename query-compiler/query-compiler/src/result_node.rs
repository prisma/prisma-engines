use indexmap::{Equivalent, IndexMap};
use query_structure::PrismaValueType;
use serde::Serialize;
use std::borrow::Borrow;
use std::hash::Hash;
use indexmap::map::Entry;

#[derive(Debug, Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum ResultNode {
    Object {
        fields: IndexMap<String, ResultNode>,
    },
    Value {
        db_name: String,
        result_type: PrismaValueType,
    },
}

impl ResultNode {
    pub fn new_object() -> Self {
        ResultNode::Object {
            fields: IndexMap::new(),
        }
    }

    pub fn new_value(db_name: String, result_type: PrismaValueType) -> Self {
        ResultNode::Value { db_name, result_type }
    }

    pub fn add_field(&mut self, key: impl Into<String>, node: ResultNode) -> Option<ResultNode> {
        match self {
            ResultNode::Object { fields } => fields.insert(key.into(), node),
            ResultNode::Value { .. } => {
                panic!("Cannot add fields to a value");
            }
        }
    }

    pub fn get_field(&self, key: impl Equivalent<String> + Hash) -> Option<&ResultNode> {
        match self {
            ResultNode::Object { fields } => fields.get(&key),
            ResultNode::Value { .. } => {
                panic!("Cannot add fields to a value");
            }
        }
    }

    pub fn get_mut_field(&mut self, key: impl Equivalent<String> + Hash) -> Option<&mut ResultNode> {
        match self {
            ResultNode::Object { fields } => fields.get_mut(&key),
            ResultNode::Value { .. } => {
                panic!("Cannot add fields to a value");
            }
        }
    }

    pub fn get_entry(&mut self, key: impl Into<String>) -> Entry<'_, String, ResultNode> {
        match self {
            ResultNode::Object { fields } => fields.entry(key.into()),
            ResultNode::Value { .. } => {
                panic!("Cannot add fields to a value");
            }
        }
    }
}
