use log::debug;
use prisma_query::connector::Queryable;
use sql_connection::{Postgresql, SyncSqlConnection};
use sql_schema_describer::*;
use std::sync::Arc;

use super::SCHEMA;

pub fn get_postgres_describer(sql: &str) -> postgres::SqlSchemaDescriber {
    let host = match std::env::var("IS_BUILDKITE") {
        Ok(_) => "test-db-postgres",
        Err(_) => "127.0.0.1",
    };

    let url = format!("postgres://postgres:prisma@{}:5432/postgres", host);
    let client = sql_connection::Postgresql::new_unpooled(url.parse().unwrap()).unwrap();

    let drop_schema = format!("DROP SCHEMA IF EXISTS \"{}\" CASCADE;", SCHEMA);
    client
        .execute_raw("", drop_schema.as_str(), &[])
        .expect("dropping schema");

    debug!("Creating Postgres schema '{}'", SCHEMA);
    client
        .execute_raw("", format!("CREATE SCHEMA \"{}\";", SCHEMA).as_str(), &[])
        .expect("creating schema");

    let sql_string = sql.to_string();
    let statements: Vec<&str> = sql_string.split(";").filter(|s| !s.is_empty()).collect();
    for statement in statements {
        debug!("Executing migration statement: '{}'", statement);
        client
            .execute_raw("", statement, &[])
            .expect("executing migration statement");
    }

    postgres::SqlSchemaDescriber::new(Arc::new(client))
}
