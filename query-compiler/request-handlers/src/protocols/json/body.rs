use indexmap::IndexMap;
use query_core::{
    BatchDocument, BatchDocumentTransaction, Operation, QueryDocument,
    schema::{QuerySchemaRef, QueryTag},
};
use serde::{Deserialize, Serialize};
use tracing::info_span;

use super::protocol_adapter::JsonProtocolAdapter;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", untagged)]
pub enum JsonBody {
    Single(JsonSingleQuery),
    Batch(JsonBatchQuery),
}

impl JsonBody {
    /// Convert a `JsonBody` into a `QueryDocument`.
    pub fn into_doc(self, query_schema: &QuerySchemaRef) -> crate::Result<QueryDocument> {
        let _span = info_span!("prisma:engine:into_doc").entered();
        match self {
            JsonBody::Single(query) => {
                let operation = JsonProtocolAdapter::new(query_schema).convert_single(query)?;

                Ok(QueryDocument::Single(operation))
            }
            JsonBody::Batch(query) => {
                let mut protocol_adapter = JsonProtocolAdapter::new(query_schema);
                let operations: crate::Result<Vec<Operation>> = query
                    .batch
                    .into_iter()
                    .map(|single_query| protocol_adapter.convert_single(single_query))
                    .collect();

                let transaction = if let Some(opts) = query.transaction {
                    Some(BatchDocumentTransaction::new(opts.isolation_level))
                } else {
                    None
                };

                Ok(QueryDocument::Multi(BatchDocument::new(operations?, transaction)))
            }
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JsonSingleQuery {
    pub model_name: Option<String>,
    pub action: Action,
    pub query: FieldQuery,
}

impl JsonSingleQuery {
    pub fn action(&self) -> &Action {
        &self.action
    }

    pub fn model(&self) -> Option<&String> {
        self.model_name.as_ref()
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct JsonBatchQuery {
    pub batch: Vec<JsonSingleQuery>,
    pub transaction: Option<BatchTransactionOption>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchTransactionOption {
    pub isolation_level: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct FieldQuery {
    pub arguments: Option<IndexMap<String, serde_json::Value>>,
    pub selection: SelectionSet,
}

#[derive(Debug)]
pub struct Action(QueryTag);

impl Action {
    pub fn new(query_tag: QueryTag) -> Self {
        Self(query_tag)
    }

    pub fn value(&self) -> QueryTag {
        self.0
    }
}

impl std::fmt::Display for Action {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

const ALL_SCALARS: &str = "$scalars";
const ALL_COMPOSITES: &str = "$composites";

#[derive(Debug, Deserialize)]
pub struct SelectionSet(IndexMap<String, SelectionSetValue>);

impl SelectionSet {
    pub fn new(selection_set: IndexMap<String, SelectionSetValue>) -> Self {
        Self(selection_set)
    }

    pub fn is_all_scalars(key: &str) -> bool {
        key == ALL_SCALARS
    }

    pub fn all_scalars(&self) -> bool {
        self.0.contains_key(ALL_SCALARS)
    }

    pub fn all_composites(&self) -> bool {
        self.0.contains_key(ALL_COMPOSITES)
    }

    pub fn is_all_composites(key: &str) -> bool {
        key == ALL_COMPOSITES
    }

    pub fn get_excluded_keys(&self) -> Vec<String> {
        self.0
            .iter()
            .filter_map(|(k, v)| (!v.is_selected()).then_some(k.to_owned()))
            .collect()
    }

    pub(crate) fn into_selection(self) -> impl Iterator<Item = (String, SelectionSetValue)> {
        self.0.into_iter()
    }
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", untagged)]
pub enum SelectionSetValue {
    Shorthand(bool),
    Nested(FieldQuery),
}

impl SelectionSetValue {
    pub fn is_selected(&self) -> bool {
        match self {
            SelectionSetValue::Shorthand(b) => *b,
            SelectionSetValue::Nested(_) => true,
        }
    }
}

impl<'de> Deserialize<'de> for Action {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let action = String::deserialize(deserializer)?;
        let query_tag = QueryTag::from(action.as_str());

        Ok(Action(query_tag))
    }
}

impl Serialize for Action {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.0.to_string().serialize(serializer)
    }
}

impl Serialize for SelectionSet {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.0.serialize(serializer)
    }
}
