use dmmf_crate::DataModelMetaFormat;
use query_core::schema::QuerySchema;

#[tracing::instrument(name = "prisma:engine:render_dmmf")]
pub fn render_dmmf(query_schema: &QuerySchema) -> DataModelMetaFormat {
    dmmf_crate::from_precomputed_parts(query_schema)
}
