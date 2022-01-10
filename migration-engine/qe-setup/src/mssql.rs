use connection_string::JdbcString;
use migration_core::migration_connector::{ConnectorError, ConnectorResult, DiffTarget};
use quaint::{prelude::*, single::Quaint};
use std::str::FromStr;

pub(crate) async fn mssql_setup(url: String, prisma_schema: &str) -> ConnectorResult<()> {
    let mut conn = JdbcString::from_str(&format!("jdbc:{}", url))
        .map_err(|e| ConnectorError::from_source(e, "JDBC string parse error"))?;
    let params = conn.properties_mut();

    let db_name = params.remove("database").unwrap_or_else(|| String::from("master"));
    let conn = Quaint::new(&conn.to_string()).await.unwrap();

    // Without these, our poor connection gets deadlocks if other schemas
    // are modified while we introspect.
    let allow_snapshot_isolation = format!(
        "ALTER DATABASE [{db_name}] SET ALLOW_SNAPSHOT_ISOLATION ON",
        db_name = db_name
    );

    conn.raw_cmd(&allow_snapshot_isolation).await.unwrap();

    let api = migration_core::migration_api(prisma_schema)?;
    let api = api.connector();
    api.reset().await.ok();

    conn.raw_cmd(&format!(
        "DROP SCHEMA IF EXISTS {}",
        conn.connection_info().schema_name()
    ))
    .await
    .unwrap();

    conn.raw_cmd(&format!("CREATE SCHEMA {}", conn.connection_info().schema_name(),))
        .await
        .unwrap();

    // 2. create the database schema for given Prisma schema
    {
        let ast = datamodel::parse_schema_ast(prisma_schema).unwrap();
        let schema = datamodel::parse_schema_parserdb(prisma_schema, &ast).unwrap();
        let migration = api
            .diff(DiffTarget::Empty, DiffTarget::Datamodel(&schema))
            .await
            .unwrap();
        api.database_migration_step_applier()
            .apply_migration(&migration)
            .await
            .unwrap();
    };
    Ok(())
}
