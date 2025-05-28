use indexmap::IndexMap;
use indexmap::map::Entry;
use query_structure::PrismaValueType;
use serde::Serialize;

#[derive(Debug, Serialize)]
#[serde(tag = "type")]
pub enum ResultNode {
    AffectedRows,
    #[serde(rename_all = "camelCase")]
    Object {
        flattened: bool,
        fields: IndexMap<String, ResultNode>,
    },
    #[serde(rename_all = "camelCase")]
    Value {
        db_name: String,
        result_type: PrismaValueType,
    },
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

    pub fn new_value(db_name: String, result_type: PrismaValueType) -> Self {
        ResultNode::Value { db_name, result_type }
    }

    pub fn add_field(&mut self, key: impl Into<String>, node: ResultNode) -> Option<ResultNode> {
        match self {
            ResultNode::Object { fields, .. } => fields.insert(key.into(), node),
            _ => panic!("Only object nodes can be indexed"),
        }
    }

    pub fn entry(&mut self, key: impl Into<String>) -> Entry<'_, String, ResultNode> {
        match self {
            ResultNode::Object { fields, .. } => fields.entry(key.into()),
            _ => panic!("Only object nodes can be indexed"),
        }
    }
}
