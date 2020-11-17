use super::test_api::mssql_2019_test_api;
use quaint::prelude::Queryable;
use sql_schema_describer::*;
use test_setup::TestAPIArgs;
use tracing::debug;

#[allow(dead_code)]
pub async fn get_mssql_describer_for_schema(sql: &str, schema: &'static str) -> mssql::SqlSchemaDescriber {
    let api = mssql_2019_test_api(TestAPIArgs::new(schema, 0b01000000)).await;
    debug!("Executing SQL Server migrations: {}", sql);

    api.database().raw_cmd(sql).await.unwrap();

    mssql::SqlSchemaDescriber::new(api.database().clone())
}
