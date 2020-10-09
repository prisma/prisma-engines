#![allow(unused)]

use quaint::{prelude::*, single::Quaint};
use sql_schema_describer::*;
use std::path::Path;
use std::sync::Arc;
use tracing::debug;

use super::SCHEMA;

pub async fn get_sqlite_describer(sql: &str, db_name: &str) -> sqlite::SqlSchemaDescriber {
    let database_folder_path = format!("{}/db", test_setup::server_root());
    let database_file_path = format!("{}/{}.db", database_folder_path, db_name);
    debug!("Database file path: '{}'", database_file_path);

    if Path::new(&database_file_path).exists() {
        std::fs::remove_file(database_file_path.clone()).expect("remove database file");
    }

    let conn = Quaint::new(&format!("file://{}?db_name={}", database_file_path, SCHEMA))
        .await
        .unwrap();

    for statement in sql.split(';').filter(|statement| !statement.is_empty()) {
        conn.query_raw(statement, &[]).await.expect("executing migration");
    }

    sqlite::SqlSchemaDescriber::new(conn)
}
