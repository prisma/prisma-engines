#![allow(unused)]

use quaint::{prelude::*, single::Quaint};
use sql_schema_describer::*;
use std::path::Path;
use std::sync::Arc;
use tracing::debug;

pub async fn get_sqlite_describer(sql: &str, db_name: &str) -> sqlite::SqlSchemaDescriber {
    let conn = Quaint::new_in_memory(Some(db_name.into())).unwrap();

    for statement in sql.split(';').filter(|statement| !statement.is_empty()) {
        conn.query_raw(statement, &[]).await.expect("executing migration");
    }

    sqlite::SqlSchemaDescriber::new(conn)
}
