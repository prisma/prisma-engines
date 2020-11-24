#![allow(unused)]

use quaint::prelude::*;
use sql_schema_describer::*;
use std::sync::Arc;
use tracing::debug;

use super::SCHEMA;

pub async fn get_postgres_describer(sql: &str, db_name: &str) -> postgres::SqlSchemaDescriber {
    let host = match std::env::var("IS_BUILDKITE") {
        Ok(_) => "test-db-postgres-10",
        Err(_) => "127.0.0.1",
    };

    let url = format!("postgres://postgres:prisma@{}:5432/{}", host, db_name);
    let client = test_setup::create_postgres_database(&url.parse().unwrap())
        .await
        .unwrap();

    let drop_schema = format!("DROP SCHEMA IF EXISTS \"{}\" CASCADE;", SCHEMA);
    client.raw_cmd(&drop_schema).await.expect("dropping schema");

    debug!("Creating Postgres schema '{}'", SCHEMA);
    client
        .raw_cmd(format!("CREATE SCHEMA \"{}\";", SCHEMA).as_str())
        .await
        .expect("creating schema");

    client.raw_cmd(sql).await.expect("executing migration");

    postgres::SqlSchemaDescriber::new(client)
}
