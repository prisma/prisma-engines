use log::debug;

use quaint::prelude::*;
use sql_schema_describer::*;
use std::sync::Arc;

fn mysql_url(schema: &str) -> String {
    let host = match std::env::var("IS_BUILDKITE") {
        Ok(_) => "test-db-mysql-5-7",
        Err(_) => "127.0.0.1",
    };
    let port = 3306;
    let user = "root";
    let password = "prisma";

    debug!("Connecting to MySQL server at {}, port {}, user '{}'", host, port, user);

    format!(
        "mysql://{user}:{password}@{host}:{port}/{schema}",
        user = user,
        password = password,
        host = host,
        port = port,
        schema = schema
    )
}

pub async fn get_mysql_describer_for_schema(sql: &str, schema: &str) -> mysql::SqlSchemaDescriber {
    // Ensure the presence of an empty database.

    let url = mysql_url(schema);
    let conn = test_setup::create_mysql_database(&url.parse().unwrap()).await.unwrap();

    // Migrate the database we just created.

    debug!("Executing MySQL migrations: {}", sql);
    let statements = sql.split(";").filter(|s| !s.is_empty());
    for statement in statements {
        debug!("Executing migration statement: '{}'", statement);
        conn.execute_raw(&statement, &[])
            .await
            .expect("executing migration statement");
    }

    mysql::SqlSchemaDescriber::new(Arc::new(conn))
}
