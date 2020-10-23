mod schema;

use query_core::schema::{QuerySchemaRef, QuerySchemaRenderer};
use schema::*;
use serde::{ser::SerializeMap, Serialize, Serializer};
use std::cmp::Ordering;
use std::{cell::RefCell, collections::HashMap};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DataModelMetaFormat {
    #[serde(rename = "datamodel")]
    pub data_model: serde_json::Value,
    pub schema: DmmfSchema,
    pub mappings: DmmfOperationMappings,
}

/// Model operations are serialized as an array of objects, each one
/// corresponding to a model. All other operations are simply an object
/// of read and write operation names that can be called on the engine.
#[derive(Debug, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct DmmfOperationMappings {
    pub model_operations: Vec<DmmfModelOperations>,
    pub other_operations: DmmfNonModelOperations,
}

#[derive(Debug, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct DmmfNonModelOperations {
    pub read: Vec<String>,
    pub write: Vec<String>,
}

#[derive(Debug, Default)]
pub struct DmmfModelOperations {
    model_name: String,
    operations: RefCell<HashMap<String, String>>,
}

impl DmmfModelOperations {
    fn new(model_name: String) -> Self {
        Self {
            model_name,
            operations: RefCell::new(HashMap::new()),
        }
    }

    fn add_operation(&self, key: String, value: String) {
        self.operations.borrow_mut().insert(key, value);
    }

    fn finalize(&self) -> HashMap<String, String> {
        // Cloning required to make the custom serializer work.
        let mut map = self.operations.borrow().clone();

        map.insert("model".into(), self.model_name.clone());
        map
    }
}

/// Serializes a DmmfMapping into a single map.
impl Serialize for DmmfModelOperations {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let finalized = self.finalize();
        let mut finalized: Vec<(String, String)> = finalized.into_iter().collect();

        // Sugar only: model is the first field in the JSON.
        finalized.sort_unstable_by(|a, b| {
            if a.0 == "model" {
                Ordering::Less
            } else if b.0 == "model" {
                Ordering::Greater
            } else {
                a.0.partial_cmp(&b.0).unwrap()
            }
        });

        let mut map = serializer.serialize_map(Some(finalized.len()))?;
        for (k, v) in finalized {
            map.serialize_entry(&k, &v)?;
        }

        map.end()
    }
}

pub fn render_dmmf(dml: &datamodel::Datamodel, query_schema: QuerySchemaRef) -> DataModelMetaFormat {
    let (schema, mappings) = DmmfQuerySchemaRenderer::render(query_schema);
    let datamodel_json = datamodel::json::dmmf::render_to_dmmf_value(&dml);

    DataModelMetaFormat {
        data_model: datamodel_json,
        schema,
        mappings,
    }
}
