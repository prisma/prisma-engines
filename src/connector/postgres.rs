mod conversion;
mod error;

use crate::{
    ast::{Id, ParameterizedValue, Query},
    connector::{queryable::*, ResultSet, Transaction},
    visitor::{self, Visitor},
    error::Error,
};
use native_tls::TlsConnector;
use tokio_postgres_native_tls::MakeTlsConnector;
use url::Url;
use tokio_postgres::config::SslMode;
use std::{str::FromStr, convert::TryFrom};

/// A connector interface for the PostgreSQL database.
pub struct PostgreSql {
    client: postgres::Client,
}

pub struct PostgresParams {
    pub connection_limit: u32,
    pub config: postgres::Config,
}

impl TryFrom<Url> for PostgresParams {
    type Error = Error;

    fn try_from(mut url: Url) -> crate::Result<Self> {
        let official = vec![];

        let (supported, unsupported): (Vec<(String, String)>, Vec<(String, String)>) = url
            .query_pairs()
            .into_iter()
            .map(|(k, v)| (String::from(k), String::from(v)))
            .collect::<Vec<(String, String)>>()
            .into_iter()
            .partition(|(k, _)| official.contains(&k.as_str()));

        url.query_pairs_mut().clear();

        supported.into_iter().for_each(|(k, v)| {
            url.query_pairs_mut().append_pair(&k, &v);
        });

        let mut config = postgres::Config::from_str(&url.to_string())?;
        let mut connection_limit = 1;

        for (k, v) in unsupported.into_iter() {
            match k.as_ref() {
                "sslmode" => {
                    match v.as_ref() {
                        "disable" => config.ssl_mode(SslMode::Disable),
                        "prefer" => config.ssl_mode(SslMode::Prefer),
                        "require" => config.ssl_mode(SslMode::Require),
                        _ => {
                            debug!("Unsupported ssl mode {}, defaulting to 'prefer'", v);
                            config.ssl_mode(SslMode::Prefer)
                        }
                    };
                }
                "connection_limit" => {
                    let as_int: u32 = v.parse().map_err(|_| Error::InvalidConnectionArguments)?;
                    connection_limit = as_int;
                }
                _ => trace!("Discarding connection string param: {}", k),
            };
        }

        Ok(Self {
            connection_limit,
            config,
        })
    }
}

impl From<postgres::Client> for PostgreSql {
    fn from(client: postgres::Client) -> Self {
        Self { client }
    }
}

impl PostgreSql {
    pub fn new(config: postgres::Config) -> crate::Result<Self> {
        let mut tls_builder = TlsConnector::builder();
        tls_builder.danger_accept_invalid_certs(true); // For Heroku

        let tls = MakeTlsConnector::new(tls_builder.build()?);
        let client = config.connect(tls)?;

        Ok(Self { client })
    }

    pub fn from_url(mut url: Url) -> crate::Result<Self> {
        let params = PostgresParams::try_from(url)?;
        Self::new(params.config)
    }
}

impl Queryable for PostgreSql {
    fn execute<'a>(&mut self, q: Query<'a>) -> crate::Result<Option<Id>> {
        let (sql, params) = dbg!(visitor::Postgres::build(q));

        let stmt = self.client.prepare(&sql)?;
        let rows = self
            .client
            .query(&stmt, &conversion::conv_params(&params))?;

        let id: Option<Id> = rows.into_iter().rev().next().map(|row| {
            let id: Id = row.get(0);
            id
        });

        Ok(id)
    }

    fn query<'a>(&mut self, q: Query<'a>) -> crate::Result<ResultSet> {
        let (sql, params) = dbg!(visitor::Postgres::build(q));
        self.query_raw(sql.as_str(), &params[..])
    }

    fn query_raw<'a>(
        &mut self,
        sql: &str,
        params: &[ParameterizedValue<'a>],
    ) -> crate::Result<ResultSet> {
        let stmt = self.client.prepare(&sql)?;
        let rows = self.client.query(&stmt, &conversion::conv_params(params))?;

        let mut result = ResultSet::new(stmt.to_column_names(), Vec::new());

        for row in rows {
            result.rows.push(row.to_result_row()?);
        }

        Ok(result)
    }

    fn execute_raw<'a>(
        &mut self,
        sql: &str,
        params: &[ParameterizedValue<'a>],
    ) -> crate::Result<u64> {
        let stmt = self.client.prepare(&sql)?;
        let changes = self
            .client
            .execute(&stmt, &conversion::conv_params(params))?;

        Ok(changes)
    }

    fn turn_off_fk_constraints(&mut self) -> crate::Result<()> {
        self.query_raw("SET CONSTRAINTS ALL DEFERRED", &[])?;
        Ok(())
    }

    fn turn_on_fk_constraints(&mut self) -> crate::Result<()> {
        self.query_raw("SET CONSTRAINTS ALL IMMEDIATE", &[])?;
        Ok(())
    }

    fn start_transaction<'b>(&'b mut self) -> crate::Result<Transaction<'b>> {
        Ok(Transaction::new(self)?)
    }

    fn raw_cmd(&mut self, cmd: &str) -> crate::Result<()> {
        self.client.simple_query(cmd)?;
        Ok(())
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

        let res = connection
            .query_raw(
                "select * from \"pg_catalog\".\"pg_am\" where amtype = 'x'",
                &[],
            )
            .unwrap();

        // No results expected.
        assert!(res.is_empty());
    }

    #[test]
    fn should_provide_a_database_transaction() {
        let mut connection = PostgreSql::new(get_config()).unwrap();
        let mut tx = connection.start_transaction().unwrap();

        let res = tx
            .query_raw(
                "select * from \"pg_catalog\".\"pg_am\" where amtype = 'x'",
                &[],
            )
            .unwrap();

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
