mod connection_like;
mod conversion;
mod error;

use crate::{
    ast::{Id, ParameterizedValue, Query},
    connector::{queryable::*, ResultSet},
    error::Error,
};
use connection_like::*;
use native_tls::TlsConnector;
use r2d2_postgres::PostgresConnectionManager;
use std::convert::TryFrom;
use tokio_postgres_native_tls::MakeTlsConnector;

type Manager = PostgresConnectionManager<MakeTlsConnector>;
type Pool = r2d2::Pool<Manager>;

/// A connector interface for the PostgreSQL database.
pub struct PostgreSql {
    pool: Pool,
}

impl Transactional for PostgreSql {
    type Error = Error;

    fn with_transaction<F, T>(&self, _db: &str, f: F) -> crate::Result<T>
    where
        F: FnOnce(&mut Queryable) -> crate::Result<T>,
    {
        let mut client = self.pool.get()?;
        let tx = client.transaction()?;
        let mut conn_like = ConnectionLike::from(tx);
        let result = f(&mut conn_like);

        if result.is_ok() {
            let tx = postgres::Transaction::try_from(conn_like).unwrap();
            tx.commit()?;
        }

        result
    }
}

impl Database for PostgreSql {
    fn with_connection<F, T>(&self, _db: &str, f: F) -> crate::Result<T>
    where
        F: FnOnce(&mut Queryable) -> crate::Result<T>,
        Self: Sized,
    {
        f(&mut ConnectionLike::from(self.pool.get()?))
    }

    fn execute_on_connection<'a>(&self, db: &str, query: Query<'a>) -> crate::Result<Option<Id>> {
        self.with_connection(&db, |conn| conn.execute(query))
    }

    fn query_on_connection<'a>(&self, db: &str, query: Query<'a>) -> crate::Result<ResultSet> {
        self.with_connection(&db, |conn| conn.query(query))
    }

    fn query_on_raw_connection<'a>(
        &self,
        db: &str,
        sql: &str,
        params: &[ParameterizedValue<'a>],
    ) -> crate::Result<ResultSet> {
        self.with_connection(&db, |conn| conn.query_raw(&sql, &params))
    }
}

impl PostgreSql {
    pub fn new(config: postgres::Config, connections: u32) -> crate::Result<PostgreSql> {
        let mut tls_builder = TlsConnector::builder();
        tls_builder.danger_accept_invalid_certs(true); // For Heroku
        let tls = MakeTlsConnector::new(tls_builder.build()?);

        let manager = PostgresConnectionManager::new(config, tls);
        let pool = r2d2::Pool::builder().max_size(connections).build(manager)?;

        Ok(PostgreSql { pool })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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
        let connector = PostgreSql::new(get_config(), 1).unwrap();

        connector
            .with_connection("TEST", |connection| {
                let res = connection.query_raw(
                    "select * from \"pg_catalog\".\"pg_am\" where amtype = 'x'",
                    &[],
                )?;

                // No results expected.
                assert!(res.is_empty());

                Ok(())
            })
            .unwrap()
    }

    #[test]
    fn should_provide_a_database_transaction() {
        let connector = PostgreSql::new(get_config(), 1).unwrap();

        connector
            .with_transaction("TEST", |transaction| {
                let res = transaction.query_raw(
                    "select * from \"pg_catalog\".\"pg_am\" where amtype = 'x'",
                    &[],
                )?;

                // No results expected.
                assert!(res.is_empty());

                Ok(())
            })
            .unwrap()
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
        let connector = PostgreSql::new(get_config(), 1).unwrap();

        connector
            .with_connection("TEST", |connection| {
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

                Ok(())
            })
            .unwrap()
    }
}
