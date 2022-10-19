use connection_string::JdbcString;
use migration_core::migration_connector::{ConnectorError, ConnectorResult};
use quaint::{prelude::*, single::Quaint};
use std::str::FromStr;

pub(crate) async fn mssql_setup(url: String, prisma_schema: &str, db_schemas: &[&str]) -> ConnectorResult<()> {
    let mut conn = JdbcString::from_str(&format!("jdbc:{}", url))
        .map_err(|e| ConnectorError::from_source(e, "JDBC string parse error"))?;
    let params = conn.properties_mut();

    let db_name = params.remove("database").unwrap_or_else(|| String::from("master"));
    let conn = Quaint::new(&conn.to_string()).await.unwrap();

    if !db_schemas.is_empty() {
        let sql = format!(
            r#"
            DROP DATABASE IF EXISTS [{db_name}];
            CREATE DATABASE [{db_name}];
            "#
        );
        conn.raw_cmd(&sql).await.unwrap();
        conn.raw_cmd(&format!("USE [{db_name}];")).await.unwrap();
    } else {
        let api = migration_core::migration_api(Some(prisma_schema.to_owned()), None)?;
        api.reset().await.ok();
        // Without these, our poor connection gets deadlocks if other schemas
        // are modified while we introspect.
        let allow_snapshot_isolation = format!(
            "ALTER DATABASE [{db_name}] SET ALLOW_SNAPSHOT_ISOLATION ON",
            db_name = db_name
        );
        conn.raw_cmd(&allow_snapshot_isolation).await.unwrap();

        conn.raw_cmd(&format!(
            "DROP SCHEMA IF EXISTS {}",
            conn.connection_info().schema_name()
        ))
        .await
        .unwrap();

        conn.raw_cmd(&format!("CREATE SCHEMA {}", conn.connection_info().schema_name(),))
            .await
            .unwrap();
    }

    // 2. create the database schema for given Prisma schema
    crate::diff_and_apply(prisma_schema).await;
    Ok(())
}
