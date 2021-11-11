mod to_dmmf;
pub use to_dmmf::render_to_dmmf;
pub use to_dmmf::render_to_dmmf_value;

// This is a simple JSON serialization using Serde.
// The JSON format follows the DMMF spec.
#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Field {
    pub name: String,
    pub kind: String,
    pub is_list: bool,
    pub is_required: bool,
    pub is_unique: bool,
    pub is_id: bool,
    pub is_read_only: bool,
    #[serde(rename = "type")]
    pub field_type: String,
    pub has_default_value: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub relation_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub relation_from_fields: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub relation_to_fields: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub relation_on_delete: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_generated: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_updated_at: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub documentation: Option<String>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Function {
    pub name: String,
    pub args: Vec<serde_json::Value>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Model {
    pub name: String,
    pub db_name: Option<String>,
    pub fields: Vec<Field>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_generated: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub documentation: Option<String>,
    pub primary_key: Option<PrimaryKey>,
    pub unique_fields: Vec<Vec<String>>,
    pub unique_indexes: Vec<UniqueIndex>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UniqueIndex {
    pub name: Option<String>,
    pub fields: Vec<String>,
}

//TODO(extended indices) add field options here
#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PrimaryKey {
    pub name: Option<String>,
    pub fields: Vec<String>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Enum {
    pub name: String,
    pub values: Vec<EnumValue>,
    pub db_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub documentation: Option<String>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EnumValue {
    pub name: String,
    pub db_name: Option<String>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct Datamodel {
    pub enums: Vec<Enum>,
    pub models: Vec<Model>,
}
