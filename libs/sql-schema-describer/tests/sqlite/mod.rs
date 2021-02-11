#![allow(unused)]

use quaint::{prelude::*, single::Quaint};
use sql_schema_describer::*;
use std::path::Path;
use std::sync::Arc;
use tracing::debug;

pub async fn get_sqlite_describer(sql: &str) -> sqlite::SqlSchemaDescriber {
    let conn = Quaint::new_in_memory().unwrap();

    conn.raw_cmd(sql).await.unwrap();

    sqlite::SqlSchemaDescriber::new(conn)
}
