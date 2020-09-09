use tracing::debug;

use super::test_api::mssql_2019_test_api;
use sql_schema_describer::*;

#[allow(dead_code)]
pub async fn get_mssql_describer_for_schema(sql: &str, schema: &'static str) -> mssql::SqlSchemaDescriber {
    let api = mssql_2019_test_api(schema).await;
    debug!("Executing SQL Server migrations: {}", sql);

    let statements = sql.split(";").filter(|s| !s.is_empty());

    for statement in statements {
        debug!("Executing migration statement: '{}'", statement);

        api.database()
            .query_raw(&statement, &[])
            .await
            .expect("executing migration statement");
    }

    mssql::SqlSchemaDescriber::new(api.database().clone())
}
