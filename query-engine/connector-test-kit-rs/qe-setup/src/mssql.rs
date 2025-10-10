use connection_string::JdbcString;
use schema_connector::{Namespaces, SchemaFilter};
use schema_core::schema_connector::{ConnectorError, ConnectorParams, ConnectorResult, SchemaConnector};
use std::str::FromStr;

pub(crate) async fn mssql_setup(url: String, prisma_schema: &str, db_schemas: &[&str]) -> ConnectorResult<()> {
    let mut conn = JdbcString::from_str(&format!("jdbc:{url}"))
        .map_err(|e| ConnectorError::from_source(e, "JDBC string parse error"))?;
    let params = conn.properties_mut();

    let db_name = params.remove("database").unwrap_or_else(|| String::from("master"));
    let schema = params.remove("schema").unwrap_or_else(|| String::from("dbo"));
    let params = ConnectorParams::new(url, Default::default(), None);
    let mut conn = sql_schema_connector::SqlSchemaConnector::new_mssql(params)?;

    if !db_schemas.is_empty() {
        let sql = format!(
            r#"
            DROP DATABASE IF EXISTS [{db_name}];
            CREATE DATABASE [{db_name}];
            "#
        );
        conn.raw_cmd(&sql).await.unwrap();
    } else {
        let ns = Namespaces::from_vec(&mut db_schemas.iter().map(|s| s.to_string()).collect());
        conn.reset(false, ns, &SchemaFilter::default()).await.ok();
        // Without these, our poor connection gets deadlocks if other schemas
        // are modified while we introspect.
        let allow_snapshot_isolation = format!("ALTER DATABASE [{db_name}] SET ALLOW_SNAPSHOT_ISOLATION ON");
        conn.raw_cmd(&allow_snapshot_isolation).await.unwrap();

        conn.raw_cmd(&format!("DROP SCHEMA IF EXISTS {}", schema))
            .await
            .unwrap();

        conn.raw_cmd(&format!("CREATE SCHEMA {}", schema)).await.unwrap();
    }

    crate::diff_and_apply(prisma_schema, &mut conn).await
}
