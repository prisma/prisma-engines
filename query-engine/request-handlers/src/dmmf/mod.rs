use dmmf_crate::DataModelMetaFormat;
use query_core::schema::QuerySchemaRef;

pub fn render_dmmf(query_schema: QuerySchemaRef) -> DataModelMetaFormat {
    dmmf_crate::from_precomputed_parts(query_schema)
}
