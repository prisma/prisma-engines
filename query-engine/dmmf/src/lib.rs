mod ast_builders;
mod serialization_ast;

#[cfg(test)]
mod tests;

pub use serialization_ast::DataModelMetaFormat;

use ast_builders::{schema_to_dmmf, DmmfQuerySchemaRenderer};
use schema::{QuerySchemaRef, QuerySchemaRenderer};
use std::sync::Arc;

pub fn dmmf_json_from_schema(schema: &str) -> String {
    let dmmf = dmmf_from_schema(schema);
    serde_json::to_string(&dmmf).unwrap()
}

pub fn dmmf_from_schema(schema: &str) -> DataModelMetaFormat {
    let schema = Arc::new(psl::parse_schema(schema).unwrap());
    let internal_data_model = prisma_models::convert(schema);
    from_precomputed_parts(Arc::new(schema_builder::build(internal_data_model, true)))
}

pub fn from_precomputed_parts(query_schema: QuerySchemaRef) -> DataModelMetaFormat {
    let data_model = schema_to_dmmf(&query_schema.internal_data_model.schema);
    let (schema, mappings) = DmmfQuerySchemaRenderer::render(query_schema);

    DataModelMetaFormat {
        data_model,
        schema,
        mappings,
    }
}
