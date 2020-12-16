use quaint::{prelude::Queryable, single::Quaint};
use sql_schema_describer::*;
use test_setup::mssql_2019_url;
use tracing::debug;

#[allow(dead_code)]
pub async fn get_mssql_describer_for_schema(sql: &str, schema: &'static str) -> mssql::SqlSchemaDescriber {
    let connection_string = format!("{};schema={}", mssql_2019_url("master"), schema);
    let conn = Quaint::new(&connection_string).await.unwrap();

    test_setup::connectors::mssql::reset_schema(&conn, schema)
        .await
        .unwrap();

    debug!("Executing SQL Server migrations: {}", sql);

    conn.raw_cmd(sql).await.unwrap();

    mssql::SqlSchemaDescriber::new(conn)
}
