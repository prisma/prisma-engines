use indexmap::IndexMap;
use indexmap::map::Entry;
use query_structure::PrismaValueType;
use serde::Serialize;

#[derive(Debug, Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum ResultNode {
    #[serde(rename_all = "camelCase")]
    Object { fields: IndexMap<String, ResultNode> },
    #[serde(rename_all = "camelCase")]
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

    pub fn get_entry(&mut self, key: impl Into<String>) -> Entry<'_, String, ResultNode> {
        match self {
            ResultNode::Object { fields } => fields.entry(key.into()),
            ResultNode::Value { .. } => {
                panic!("Cannot add fields to a value");
            }
        }
    }
}
