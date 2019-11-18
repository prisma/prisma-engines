use log::debug;
use sql_connection::{GenericSqlConnection, Queryable};
use sql_schema_describer::*;
use std::sync::Arc;

use super::SCHEMA;

fn mysql_url(schema: &str) -> String {
    let host = match std::env::var("IS_BUILDKITE") {
        Ok(_) => "test-db-mysql-5-7",
        Err(_) => "127.0.0.1",
    };
    let port = 3306;
    let user = "root";
    let password = "prisma";

    debug!("Connecting to MySQL server at {}, port {}, user '{}'", host, port, user);

    format!(
        "mysql://{user}:{password}@{host}:{port}/{schema}",
        user = user,
        password = password,
        host = host,
        port = port,
        schema = schema
    )
}

pub async fn get_mysql_describer(sql: &str) -> mysql::SqlSchemaDescriber {
    // Ensure the presence of an empty database.

    let url = mysql_url("");
    let conn = GenericSqlConnection::from_database_str(&url, None).unwrap();

    conn.execute_raw(&format!("DROP SCHEMA IF EXISTS `{}`", SCHEMA), &[])
        .await
        .expect("dropping schema");
    conn.execute_raw(&format!("CREATE SCHEMA `{}`", SCHEMA), &[])
        .await
        .expect("creating schema");

    // Migrate the database we just created.

    let url = mysql_url(SCHEMA);
    let conn = GenericSqlConnection::from_database_str(&url, None).unwrap();

    debug!("Executing MySQL migrations: {}", sql);
    let sql_string = sql.to_string();
    let statements: Vec<&str> = sql_string.split(";").filter(|s| !s.is_empty()).collect();
    for statement in statements {
        debug!("Executing migration statement: '{}'", statement);
        conn.execute_raw(&statement, &[])
            .await
            .expect("executing migration statement");
    }

    mysql::SqlSchemaDescriber::new(Arc::new(conn))
}
