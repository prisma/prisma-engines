mod ast_builders;
mod serialization_ast;

#[cfg(test)]
mod tests;

pub use serialization_ast::DataModelMetaFormat;

use ast_builders::schema_to_dmmf;
use schema::QuerySchema;
use std::sync::Arc;

pub fn dmmf_json_from_schema(schema: &str) -> String {
    let dmmf = dmmf_from_schema(schema);
    serde_json::to_string(&dmmf).unwrap()
}

#[cfg_attr(not(target_arch = "wasm32"), allow(clippy::useless_conversion))]
pub fn dmmf_from_schema(schema: &str) -> DataModelMetaFormat {
    let schema = psl::parse_schema(schema).unwrap();
    from_precomputed_parts(&schema::build(Arc::new(schema.into()), true))
}

pub fn from_precomputed_parts(query_schema: &QuerySchema) -> DataModelMetaFormat {
    let data_model = schema_to_dmmf(&query_schema.internal_data_model.schema);
    let (schema, mappings) = ast_builders::render(query_schema);

    DataModelMetaFormat {
        data_model,
        schema,
        mappings,
    }
}
