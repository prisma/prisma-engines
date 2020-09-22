use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct DmmfSchema {
    pub root_query_type: String,
    pub root_mutation_type: String,
    pub input_types: Vec<DmmfInputType>,
    pub output_types: Vec<DmmfOutputType>,
    pub enums: Vec<DmmfEnum>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DmmfOutputField {
    pub name: String,
    pub args: Vec<DmmfInputField>,
    pub is_required: bool,
    pub is_nullable: bool,
    pub output_type: DmmfTypeReference,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DmmfInputType {
    pub name: String,
    pub constraints: DmmfInputTypeConstraints,
    pub fields: Vec<DmmfInputField>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DmmfInputTypeConstraints {
    pub max_num_fields: Option<usize>,
    pub min_num_fields: Option<usize>,
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
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DmmfTypeReference {
    #[serde(rename = "type")]
    pub typ: String,
    pub kind: TypeKind,
    pub is_list: bool,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TypeKind {
    Scalar,
    Object,
    Enum,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DmmfEnum {
    pub name: String,
    pub values: Vec<String>,
}
