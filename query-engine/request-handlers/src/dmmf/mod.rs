use dmmf_crate::DataModelMetaFormat;
use query_core::schema::QuerySchemaRef;

pub fn render_dmmf(dml: &psl::dml::Datamodel, query_schema: QuerySchemaRef) -> DataModelMetaFormat {
    dmmf_crate::from_precomputed_parts(dml, query_schema)
}
