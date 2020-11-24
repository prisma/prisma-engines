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

    conn.raw_cmd(&sql).await.expect("executing migration");

    mysql::SqlSchemaDescriber::new(conn)
}
