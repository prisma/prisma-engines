pub mod datamodel_ast;
pub mod mappings_ast;
pub mod schema_ast;

pub use datamodel_ast::*;
pub use mappings_ast::*;
pub use schema_ast::*;

use serde::Serialize;

/// The main DMMF serialization struct.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DataModelMetaFormat {
    #[serde(rename = "datamodel")]
    /// The datamodel AST.
    pub data_model: Datamodel,

    /// The query-engine API schema.
    pub schema: DmmfSchema,

    /// The operations map. Derived from the `schema`.
    pub mappings: DmmfOperationMappings,
}
