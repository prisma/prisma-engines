use connection_string::JdbcString;
use schema_connector::SchemaFilter;
use schema_core::schema_connector::{ConnectorError, ConnectorParams, ConnectorResult, SchemaConnector};
use std::str::FromStr;

pub(crate) async fn mssql_setup(url: String, prisma_schema: &str, db_schemas: &[&str]) -> ConnectorResult<()> {
    let mut jdbc_url = JdbcString::from_str(&format!("jdbc:{url}"))
        .map_err(|e| ConnectorError::from_source(e, "JDBC string parse error"))?;
    let props = jdbc_url.properties_mut();

    let db_name = props.remove("database").unwrap_or_else(|| String::from("master"));
    let schema = props.get("schema").map_or("dbo", String::as_str).to_owned();
    let params = ConnectorParams::new(jdbc_url.to_string(), Default::default(), None);
    let mut tmp_conn = sql_schema_connector::SqlSchemaConnector::new_mssql(params)?;

    if !db_schemas.is_empty() {
        let sql = format!(
            r#"
            DROP DATABASE IF EXISTS [{db_name}];
            CREATE DATABASE [{db_name}];
            "#
        );
        tmp_conn.raw_cmd(&sql).await.unwrap();
    } else {
        tmp_conn.reset(false, None, &SchemaFilter::default()).await.ok();
        // Without these, our poor connection gets deadlocks if other schemas
        // are modified while we introspect.
        let allow_snapshot_isolation = format!("ALTER DATABASE [{db_name}] SET ALLOW_SNAPSHOT_ISOLATION ON");
        tmp_conn.raw_cmd(&allow_snapshot_isolation).await.unwrap();

        tmp_conn
            .raw_cmd(&format!("DROP SCHEMA IF EXISTS {}", schema))
            .await
            .unwrap();

        tmp_conn.raw_cmd(&format!("CREATE SCHEMA {}", schema)).await.unwrap();
    }

    tmp_conn.dispose().await.unwrap();

    let params = ConnectorParams::new(url, Default::default(), None);
    let mut conn = sql_schema_connector::SqlSchemaConnector::new_mssql(params)?;
    crate::diff_and_apply(prisma_schema, &mut conn).await
}
