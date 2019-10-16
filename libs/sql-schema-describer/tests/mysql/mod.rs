use log::debug;
use prisma_query::{ast::ParameterizedValue, connector::Queryable};
use sql_schema_describer::*;
use std::sync::{Arc, Mutex};

use super::SCHEMA;

struct MySqlConnection {
    client: Mutex<prisma_query::connector::Mysql>,
}

impl crate::SqlConnection for MySqlConnection {
    fn query_raw(
        &self,
        sql: &str,
        _: &str,
        params: &[ParameterizedValue],
    ) -> prisma_query::Result<prisma_query::connector::ResultSet> {
        self.client.lock().expect("self.client.lock").query_raw(sql, params)
    }
}

pub fn get_mysql_describer(sql: &str) -> mysql::SqlSchemaDescriber {
    let host = match std::env::var("IS_BUILDKITE") {
        Ok(_) => "test-db-mysql-5-7",
        Err(_) => "127.0.0.1",
    };
    let port = 3306;
    let user = "root";
    let password = "prisma";

    debug!("Connecting to MySQL server at {}, port {}, user '{}'", host, port, user);

    let mut opts_builder = prisma_query::pool::mysql::OptsBuilder::new();
    opts_builder
        .ip_or_hostname(Some(host))
        .tcp_port(port)
        .user(Some(user))
        .pass(Some(password));
    let mut conn = prisma_query::connector::Mysql::new(opts_builder).expect("connect to MySQL");

    conn.execute_raw(&format!("DROP SCHEMA IF EXISTS `{}`", SCHEMA), &[])
        .expect("dropping schema");
    conn.execute_raw(&format!("CREATE SCHEMA `{}`", SCHEMA), &[])
        .expect("creating schema");

    debug!("Executing MySQL migrations: {}", sql);
    let sql_string = sql.to_string();
    let statements: Vec<&str> = sql_string.split(";").filter(|s| !s.is_empty()).collect();
    for statement in statements {
        debug!("Executing migration statement: '{}'", statement);
        conn.execute_raw(&statement, &[])
            .expect("executing migration statement");
    }

    let mut opts_builder = prisma_query::pool::mysql::OptsBuilder::new();
    opts_builder
        .ip_or_hostname(Some(host))
        .tcp_port(port)
        .user(Some(user))
        .pass(Some(password))
        .db_name(Some(SCHEMA));
    let conn = prisma_query::connector::Mysql::new(opts_builder).expect("connect to MySQL");

    mysql::SqlSchemaDescriber::new(Arc::new(MySqlConnection {
        client: Mutex::new(conn),
    }))
}
