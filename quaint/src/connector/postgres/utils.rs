use indexmap::IndexMap;
use postgres_types::Json;
use serde::{Deserialize, Serialize};
use tracing::log::warn;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", untagged)]
pub enum JsonBody {
    Single(JsonSingleQuery),
    Batch(JsonBatchQuery),
}
impl JsonBody {
    fn client_shape(&self) -> String {
        serde_json::to_string_pretty(&self).unwrap()
    }
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JsonSingleQuery {
    pub model_name: Option<String>,
    pub action: String,
    pub query: FieldQuery,
}

impl JsonSingleQuery {
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

    pub(crate) fn into_selection(self) -> impl Iterator<Item = (String, SelectionSetValue)> {
        self.0.into_iter().filter(|(_, v)| v.is_selected())
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

impl Serialize for SelectionSet {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.0.serialize(serializer)
    }
}

pub fn generate_shape_and_tag(prisma_query: &str) -> (String, String) {
    let shape = if let Ok(body) = serde_json::de::from_str::<JsonBody>(prisma_query) {
        body.client_shape()
    } else {
        warn!("Failed to get shape of query: {}", prisma_query);
        prisma_query.to_string()
    };

    let mut hasher = sha1::Sha1::new();
    hasher.update(shape.as_bytes());

    let tag = hasher.digest().to_string();

    (shape, tag)
}

#[derive(Deserialize, Serialize, Debug)]
pub struct SubmittedQueryInfo {
    pub raw_query: String,
    pub tag: String,
    pub prisma_query: String,
}
