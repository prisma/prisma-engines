use schema_core::schema_connector::{ConnectorParams, ConnectorResult};

pub(crate) async fn sqlite_setup(source: psl::Datasource, url: String, prisma_schema: &str) -> ConnectorResult<()> {
    std::fs::remove_file(source.url.as_literal().unwrap().trim_start_matches("file:")).ok();
    let params = ConnectorParams::new(url, Default::default(), None);
    let mut connector = sql_schema_connector::SqlSchemaConnector::new_sqlite(params)?;
    crate::diff_and_apply(prisma_schema, &mut connector).await
}
