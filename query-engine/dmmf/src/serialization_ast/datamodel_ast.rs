// Datamodel serialization AST for the DMMF.
#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Datamodel {
    pub enums: Vec<Enum>,
    pub models: Vec<Model>,
    pub types: Vec<Model>, // composite types
    pub indexes: Vec<Index>,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Field {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub db_name: Option<String>,
    pub kind: &'static str,
    pub is_list: bool,
    pub is_required: bool,
    pub is_unique: bool,
    pub is_id: bool,
    pub is_read_only: bool,
    pub has_default_value: bool,

    #[serde(rename = "type")]
    pub field_type: String,

    pub native_type: Option<(String, Vec<String>)>,

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

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Function {
    pub name: String,
    pub args: Vec<serde_json::Value>,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Model {
    pub name: String,
    pub db_name: Option<String>,
    pub fields: Vec<Field>,
    pub primary_key: Option<PrimaryKey>,
    pub unique_fields: Vec<Vec<String>>,
    pub unique_indexes: Vec<UniqueIndex>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_generated: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub documentation: Option<String>,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UniqueIndex {
    pub name: Option<String>,
    pub fields: Vec<String>,
}

// TODO(extended indices) add field options here
#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PrimaryKey {
    pub name: Option<String>,
    pub fields: Vec<String>,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Enum {
    pub name: String,
    pub values: Vec<EnumValue>,
    pub db_name: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub documentation: Option<String>,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EnumValue {
    pub name: String,
    pub db_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub documentation: Option<String>,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Index {
    pub model: String,
    pub r#type: IndexType,
    pub is_defined_on_field: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub db_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub algorithm: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub clustered: Option<bool>,
    pub fields: Vec<IndexField>,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IndexField {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sort_order: Option<SortOrder>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub length: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub operator_class: Option<String>,
}

macro_rules! from {
    ( $from:path => $to:ident { $( $variant:ident ),+ } ) => {
        impl From<$from> for $to {
            fn from(value: $from) -> Self {
                match value {
                    $( <$from>::$variant => <$to>::$variant ),+
                }
            }
        }
    };
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub enum IndexType {
    Id,
    Normal,
    Unique,
    Fulltext,
}

// `Id` doesn't exist in `psl::parser_database::IndexType` as primary keys are not represented as
// such on that level, so we only generate the From impl for the other three variants.
from!(psl::parser_database::IndexType => IndexType { Normal, Unique, Fulltext });

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub enum SortOrder {
    Asc,
    Desc,
}

from!(psl::parser_database::SortOrder => SortOrder { Asc, Desc });
