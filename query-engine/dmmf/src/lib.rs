mod ast_builders;
mod serialization_ast;

pub use serialization_ast::DataModelMetaFormat;

use ast_builders::{schema_to_dmmf, DmmfQuerySchemaRenderer};
use prisma_models::InternalDataModelBuilder;
use schema::{QuerySchemaRef, QuerySchemaRenderer};
use std::sync::Arc;

pub fn dmmf_json_from_schema(schema: &str) -> String {
    let dmmf = dmmf_from_schema(schema);
    serde_json::to_string(&dmmf).unwrap()
}

// enable raw param?
pub fn dmmf_from_schema(schema: &str) -> DataModelMetaFormat {
    let (config, dml) = psl::parse_schema(schema).unwrap();

    // We only support one data source at the moment, so take the first one (default not exposed yet).
    let data_source = config.datasources.first().unwrap();
    let preview_features: Vec<_> = config.preview_features().iter().collect();
    let internal_data_model = InternalDataModelBuilder::from(&dml).build("dummy".to_owned());

    // Construct query schema
    let query_schema = Arc::new(schema_builder::build(
        internal_data_model,
        true, // todo
        data_source.capabilities(),
        preview_features,
        data_source.referential_integrity(),
    ));

    from_precomputed_parts(&dml, query_schema)
}

pub fn from_precomputed_parts(dml: &psl::dml::Datamodel, query_schema: QuerySchemaRef) -> DataModelMetaFormat {
    let (schema, mappings) = DmmfQuerySchemaRenderer::render(query_schema);
    let data_model = schema_to_dmmf(dml);

    DataModelMetaFormat {
        data_model,
        schema,
        mappings,
    }
}
