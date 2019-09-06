use log::debug;
use prisma_query::{ast::ParameterizedValue, connector::Queryable};
use sql_schema_describer::*;
use std::path::Path;
use std::sync::{Arc, Mutex};

use super::SCHEMA;

struct SqliteConnection {
    client: Mutex<prisma_query::connector::Sqlite>,
}

impl crate::IntrospectionConnection for SqliteConnection {
    fn query_raw(
        &self,
        sql: &str,
        _schema: &str,
        params: &[ParameterizedValue],
    ) -> prisma_query::Result<prisma_query::connector::ResultSet> {
        self.client.lock().expect("self.client.lock").query_raw(sql, params)
    }
}

pub fn get_sqlite_connector(sql: &str) -> sqlite::IntrospectionConnector {
    let server_root = std::env::var("SERVER_ROOT").expect("Env var SERVER_ROOT required but not found.");
    let database_folder_path = format!("{}/db", server_root);
    let database_file_path = format!("{}/{}.db", database_folder_path, SCHEMA);
    debug!("Database file path: '{}'", database_file_path);
    if Path::new(&database_file_path).exists() {
        std::fs::remove_file(database_file_path.clone()).expect("remove database file");
    }

    let conn = rusqlite::Connection::open_in_memory().expect("opening SQLite connection should work");
    conn.execute(
        "ATTACH DATABASE ? as ?",
        &vec![database_file_path.clone(), String::from(SCHEMA)],
    )
    .expect("attach SQLite database");
    debug!("Executing migration: {}", sql);
    conn.execute_batch(sql).expect("executing migration");
    conn.close().expect("closing SQLite connection");

    let mut queryable =
        prisma_query::connector::Sqlite::new(database_file_path).expect("opening prisma_query::connector::Sqlite");
    queryable.attach_database(SCHEMA).expect("attaching database");
    let int_conn = Arc::new(SqliteConnection {
        client: Mutex::new(queryable),
    });
    sqlite::IntrospectionConnector::new(int_conn)
}
