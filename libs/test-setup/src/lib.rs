//! This crate contains constants and utilities that are useful for writing tests across the
//! engines.

pub const SCHEMA_NAME: &str = "prisma-tests";

pub fn sqlite_test_url() -> String {
    format!("file:{}?db_name={}", sqlite_test_file(), SCHEMA_NAME)
}

pub fn sqlite_test_file() -> String {
    let server_root = std::env::var("SERVER_ROOT").expect("Env var SERVER_ROOT required but not found.");
    let database_folder_path = format!("{}/db", server_root);
    let file_path = format!("{}/{}.db", database_folder_path, SCHEMA_NAME);
    file_path
}

pub fn postgres_url() -> String {
    dbg!(format!(
        "postgresql://postgres:prisma@{}:5432/test-db?schema={}",
        db_host_postgres(),
        SCHEMA_NAME
    ))
}

pub fn mysql_url() -> String {
    dbg!(format!("mysql://root:prisma@{}:3306/", db_host_mysql_5_7()))
}

pub fn mysql_8_url() -> String {
    let (host, port) = db_host_and_port_mysql_8_0();
    dbg!(format!(
        "mysql://root:prisma@{host}:{port}/{schema_name}",
        host = host,
        port = port,
        schema_name = SCHEMA_NAME
    ))
}

fn db_host_postgres() -> String {
    match std::env::var("IS_BUILDKITE") {
        Ok(_) => "test-db-postgres".to_string(),
        Err(_) => "127.0.0.1".to_string(),
    }
}

fn db_host_and_port_mysql_8_0() -> (String, usize) {
    match std::env::var("IS_BUILDKITE") {
        Ok(_) => ("test-db-mysql-8-0".to_string(), 3306),
        Err(_) => ("127.0.0.1".to_string(), 3307),
    }
}

fn db_host_mysql_5_7() -> String {
    match std::env::var("IS_BUILDKITE") {
        Ok(_) => "test-db-mysql-5-7".to_string(),
        Err(_) => "127.0.0.1".to_string(),
    }
}
