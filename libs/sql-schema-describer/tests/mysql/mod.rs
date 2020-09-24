#![allow(unused)]

use tracing::debug;

use quaint::prelude::*;
use sql_schema_describer::*;
use std::sync::Arc;
use test_setup::mysql_url;

pub async fn get_mysql_describer_for_schema(sql: &str, schema: &str) -> mysql::SqlSchemaDescriber {
    // Ensure the presence of an empty database.

    let url = mysql_url(schema);
    let conn = test_setup::create_mysql_database(&url.parse().unwrap()).await.unwrap();

    // Migrate the database we just created.

    debug!("Executing MySQL migrations: {}", sql);
    let statements = sql.split(";").filter(|s| !s.is_empty());
    for statement in statements {
        debug!("Executing migration statement: '{}'", statement);
        conn.query_raw(&statement, &[])
            .await
            .expect("executing migration statement");
    }

    mysql::SqlSchemaDescriber::new(conn)
}
