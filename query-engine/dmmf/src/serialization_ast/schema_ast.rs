use indexmap::IndexMap;
use schema::Deprecation;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct DmmfSchema {
    pub input_object_types: IndexMap<String, Vec<DmmfInputType>>,
    pub output_object_types: IndexMap<String, Vec<DmmfOutputType>>,
    pub enum_types: IndexMap<String, Vec<DmmfEnum>>,
    pub field_ref_types: IndexMap<String, Vec<DmmfFieldRefType>>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DmmfOutputField {
    pub name: String,
    pub args: Vec<DmmfInputField>,
    pub is_nullable: bool,
    pub output_type: DmmfTypeReference,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub deprecation: Option<DmmfDeprecation>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DmmfInputType {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<DmmfInputTypeMeta>,
    pub constraints: DmmfInputTypeConstraints,
    pub fields: Vec<DmmfInputField>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DmmfInputTypeConstraints {
    pub max_num_fields: Option<usize>,
    pub min_num_fields: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fields: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DmmfInputTypeMeta {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DmmfFieldRefType {
    pub name: String,
    pub allow_types: Vec<DmmfTypeReference>,
    pub fields: Vec<DmmfInputField>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DmmfOutputType {
    pub name: String,
    pub fields: Vec<DmmfOutputField>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DmmfInputField {
    pub name: String,
    pub is_required: bool,
    pub is_nullable: bool,
    pub input_types: Vec<DmmfTypeReference>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub deprecation: Option<DmmfDeprecation>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DmmfTypeReference {
    #[serde(rename = "type")]
    pub typ: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub namespace: Option<String>,
    pub location: TypeLocation,
    pub is_list: bool,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TypeLocation {
    Scalar,
    InputObjectTypes,
    OutputObjectTypes,
    EnumTypes,
    FieldRefTypes,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DmmfEnum {
    pub name: String,
    pub values: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DmmfDeprecation {
    pub since_version: String,
    pub reason: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub planned_removal_version: Option<String>,
}

impl From<&Deprecation> for DmmfDeprecation {
    fn from(deprecation: &Deprecation) -> Self {
        Self {
            since_version: deprecation.since_version.clone(),
            planned_removal_version: deprecation.planned_removal_version.clone(),
            reason: deprecation.reason.clone(),
        }
    }
}
