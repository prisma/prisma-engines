use psl::Datasource;
use quaint::{prelude::*, single::Quaint};
use schema_core::schema_connector::ConnectorResult;

pub(crate) async fn sqlite_setup(url: String, source: Datasource, prisma_schema: &str) -> ConnectorResult<()> {
    std::fs::remove_file(source.url.as_literal().unwrap().trim_start_matches("file:")).ok();
    let mut connector = sql_schema_connector::SqlSchemaConnector::new_sqlite();
    let client = Quaint::new(&url).await.unwrap();
    client.query_raw("SELECT InitSpatialMetaData()", &[]).await.ok();
    crate::diff_and_apply(prisma_schema, url, &mut connector).await
}
