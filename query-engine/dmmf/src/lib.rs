mod ast_builders;
mod serialization_ast;

#[cfg(test)]
mod tests;

use psl::ValidatedSchema;
pub use serialization_ast::{DataModelMetaFormat, Datamodel};

use ast_builders::schema_to_dmmf;
use schema::QuerySchema;
use std::sync::Arc;

pub fn dmmf_json_from_schema(schema: &str) -> String {
    let dmmf = dmmf_from_schema(schema);
    serde_json::to_string(&dmmf).unwrap()
}

pub fn dmmf_json_from_validated_schema(schema: ValidatedSchema) -> String {
    let dmmf = from_precomputed_parts(&schema::build(Arc::new(schema), true));
    serde_json::to_string(&dmmf).unwrap()
}

pub fn dmmf_from_schema(schema: &str) -> DataModelMetaFormat {
    let schema = Arc::new(psl::parse_schema(schema).unwrap());
    from_precomputed_parts(&schema::build(schema, true))
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

#[inline]
pub fn datamodel_from_validated_schema(schema: &ValidatedSchema) -> Datamodel {
    schema_to_dmmf(schema)
}
