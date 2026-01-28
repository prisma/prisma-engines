use indexmap::IndexMap;
use serde::Serialize;

#[derive(Debug, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct DmmfSchema {
    pub input_object_types: IndexMap<String, Vec<DmmfInputType>>,
    pub output_object_types: IndexMap<String, Vec<DmmfOutputType>>,
    pub enum_types: IndexMap<String, Vec<DmmfEnum>>,
    pub field_ref_types: IndexMap<String, Vec<DmmfFieldRefType>>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DmmfOutputField {
    pub name: String,
    pub args: Vec<DmmfInputField>,
    pub is_nullable: bool,
    pub output_type: DmmfTypeReference,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub deprecation: Option<DmmfDeprecation>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DmmfInputType {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<DmmfInputTypeMeta>,
    pub constraints: DmmfInputTypeConstraints,
    pub fields: Vec<DmmfInputField>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DmmfInputTypeConstraints {
    pub max_num_fields: Option<usize>,
    pub min_num_fields: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fields: Option<Vec<String>>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DmmfInputTypeMeta {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    /// Used by the generator to group input types roughly by model.
    /// Note that it is not strictly the model name but can also be a composite type name or empty for generic input types.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub grouping: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DmmfFieldRefType {
    pub name: String,
    pub allow_types: Vec<DmmfTypeReference>,
    pub fields: Vec<DmmfInputField>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DmmfOutputType {
    pub name: String,
    pub fields: Vec<DmmfOutputField>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DmmfInputField {
    pub name: String,
    pub is_required: bool,
    pub is_nullable: bool,
    pub input_types: Vec<DmmfTypeReference>,
    pub is_parameterizable: bool,

    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub requires_other_fields: Vec<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub deprecation: Option<DmmfDeprecation>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DmmfTypeReference {
    #[serde(rename = "type")]
    pub typ: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub namespace: Option<String>,
    pub location: TypeLocation,
    pub is_list: bool,
}

#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum TypeLocation {
    Scalar,
    InputObjectTypes,
    OutputObjectTypes,
    EnumTypes,
    FieldRefTypes,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DmmfEnum {
    pub name: String,
    pub values: Vec<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DmmfDeprecation {
    pub since_version: String,
    pub reason: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub planned_removal_version: Option<String>,
}
