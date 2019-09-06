use log::debug;
use prisma_query::{ast::ParameterizedValue, connector::Queryable};
use sql_schema_describer::*;
use std::sync::{Arc, Mutex};

use super::SCHEMA;

struct PostgresConnection {
    client: Mutex<prisma_query::connector::PostgreSql>,
}

impl crate::IntrospectionConnection for PostgresConnection {
    fn query_raw(
        &self,
        sql: &str,
        _: &str,
        params: &[ParameterizedValue],
    ) -> prisma_query::Result<prisma_query::connector::ResultSet> {
        self.client.lock().expect("self.client.lock").query_raw(sql, params)
    }
}

pub fn get_postgres_connector(sql: &str) -> postgres::IntrospectionConnector {
    let host = match std::env::var("IS_BUILDKITE") {
        Ok(_) => "test-db-postgres",
        Err(_) => "127.0.0.1",
    };
    let mut client = ::postgres::Config::new()
        .user("postgres")
        .password("prisma")
        .host(host)
        .port(5432)
        .dbname("postgres")
        .connect(::postgres::NoTls)
        .expect("connecting to Postgres");

    let drop_schema = format!("DROP SCHEMA IF EXISTS \"{}\" CASCADE;", SCHEMA);
    client.execute(drop_schema.as_str(), &[]).expect("dropping schema");

    debug!("Creating Postgres schema '{}'", SCHEMA);
    client
        .execute(format!("CREATE SCHEMA \"{}\";", SCHEMA).as_str(), &[])
        .expect("creating schema");

    let sql_string = sql.to_string();
    let statements: Vec<&str> = sql_string.split(";").filter(|s| !s.is_empty()).collect();
    for statement in statements {
        debug!("Executing migration statement: '{}'", statement);
        client.execute(statement, &[]).expect("executing migration statement");
    }

    let conn = Arc::new(PostgresConnection {
        client: Mutex::new(prisma_query::connector::PostgreSql::from(client)),
    });
    postgres::IntrospectionConnector::new(conn)
}
