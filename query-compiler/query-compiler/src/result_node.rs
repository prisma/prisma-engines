use indexmap::IndexMap;
use indexmap::map::Entry;
use query_structure::{FieldArity, PrismaValueType};
use serde::Serialize;

#[derive(Debug, Serialize)]
#[serde(tag = "type")]
pub enum ResultNode {
    #[serde(rename_all = "camelCase")]
    Object {
        flattened: bool,
        fields: IndexMap<String, ResultNode>,
    },
    #[serde(rename_all = "camelCase")]
    Value {
        db_name: String,
        result_type: PrismaValueType,
        arity: ValueArity,
    },
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ValueArity {
    Required,
    Optional,
    List,
}

impl ResultNode {
    pub fn new_object() -> Self {
        ResultNode::Object {
            flattened: false,
            fields: IndexMap::new(),
        }
    }

    pub fn new_flattened_object() -> Self {
        ResultNode::Object {
            flattened: true,
            fields: IndexMap::new(),
        }
    }

    pub fn new_value(db_name: String, result_type: PrismaValueType, arity: ValueArity) -> Self {
        ResultNode::Value {
            db_name,
            result_type,
            arity,
        }
    }

    pub fn add_field(&mut self, key: impl Into<String>, node: ResultNode) -> Option<ResultNode> {
        match self {
            ResultNode::Object { fields, .. } => fields.insert(key.into(), node),
            ResultNode::Value { .. } => {
                panic!("Only object nodes can be indexed");
            }
        }
    }

    pub fn entry(&mut self, key: impl Into<String>) -> Entry<'_, String, ResultNode> {
        match self {
            ResultNode::Object { fields, .. } => fields.entry(key.into()),
            ResultNode::Value { .. } => {
                panic!("Only object nodes can be indexed");
            }
        }
    }
}

impl From<FieldArity> for ValueArity {
    fn from(arity: FieldArity) -> Self {
        match arity {
            FieldArity::Required => ValueArity::Required,
            FieldArity::Optional => ValueArity::Optional,
            FieldArity::List => ValueArity::List,
        }
    }
}
