mod connection_like;
mod conversion;
mod error;

use native_tls::TlsConnector;
use tokio_postgres_native_tls::MakeTlsConnector;

pub use connection_like::ConnectionLike;

/// A connector interface for the PostgreSQL database.
pub struct PostgreSql {
    client: postgres::Client,
}

impl From<postgres::Client> for PostgreSql {
    fn from(client: postgres::Client) -> Self {
        Self { client }
    }
}

impl PostgreSql {
    pub fn new(config: postgres::Config) -> crate::Result<ConnectionLike<Self>> {
        let mut tls_builder = TlsConnector::builder();
        tls_builder.danger_accept_invalid_certs(true); // For Heroku

        let tls = MakeTlsConnector::new(tls_builder.build()?);
        let client = config.connect(tls)?;

        Ok(ConnectionLike::from(PostgreSql { client }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::connector::Queryable;
    use std::env;

    #[allow(unused)]
    fn get_config() -> postgres::Config {
        let mut config = postgres::Config::new();
        config.host(&env::var("TEST_PG_HOST").unwrap());
        config.dbname(&env::var("TEST_PG_DB").unwrap());
        config.user(&env::var("TEST_PG_USER").unwrap());
        config.password(env::var("TEST_PG_PASSWORD").unwrap());
        config.port(env::var("TEST_PG_PORT").unwrap().parse::<u16>().unwrap());
        config
    }

    #[test]
    fn should_provide_a_database_connection() {
        let mut connection = PostgreSql::new(get_config()).unwrap();

        let res = connection.query_raw(
            "select * from \"pg_catalog\".\"pg_am\" where amtype = 'x'",
            &[],
        ).unwrap();

        // No results expected.
        assert!(res.is_empty());
    }

    #[test]
    fn should_provide_a_database_transaction() {
        let mut connection = PostgreSql::new(get_config()).unwrap();
        let mut tx = connection.start_transaction().unwrap();

        let res = tx.query_raw(
            "select * from \"pg_catalog\".\"pg_am\" where amtype = 'x'",
            &[],
        ).unwrap();

        assert!(res.is_empty());
    }

    #[allow(unused)]
    const TABLE_DEF: &str = r#"
    CREATE TABLE "user"(
        id       int4    PRIMARY KEY     NOT NULL,
        name     text    NOT NULL,
        age      int4    NOT NULL,
        salary   float4
    );
    "#;

    #[allow(unused)]
    const CREATE_USER: &str = r#"
    INSERT INTO "user" (id, name, age, salary)
    VALUES (1, 'Joe', 27, 20000.00 );
    "#;

    #[allow(unused)]
    const DROP_TABLE: &str = "DROP TABLE IF EXISTS \"user\";";

    #[test]
    fn should_map_columns_correctly() {
        let mut connection = PostgreSql::new(get_config()).unwrap();
        connection.query_raw(DROP_TABLE, &[]).unwrap();
        connection.query_raw(TABLE_DEF, &[]).unwrap();
        connection.query_raw(CREATE_USER, &[]).unwrap();

        let rows = connection.query_raw("SELECT * FROM \"user\"", &[]).unwrap();
        assert_eq!(rows.len(), 1);

        let row = rows.get(0).unwrap();
        assert_eq!(row["id"].as_i64(), Some(1));
        assert_eq!(row["name"].as_str(), Some("Joe"));
        assert_eq!(row["age"].as_i64(), Some(27));
        assert_eq!(row["salary"].as_f64(), Some(20000.0));
    }
}
