mod conversion;
mod error;

use crate::{
    ast::{Id, ParameterizedValue, Query},
    connector::{metrics, queryable::*, ResultSet, Transaction},
    error::Error,
    visitor::{self, Visitor},
};
use native_tls::TlsConnector;
use percent_encoding::percent_decode;
use std::{borrow::Borrow, convert::TryFrom, time::Duration};
use tokio_postgres::config::SslMode;
use tokio_postgres_native_tls::MakeTlsConnector;
use url::Url;

pub(crate) const DEFAULT_SCHEMA: &str = "public";

/// A connector interface for the PostgreSQL database.
#[derive(DebugStub)]
pub struct PostgreSql {
    #[debug_stub = "postgres::Client"]
    client: postgres::Client,
}

#[derive(Debug)]
pub struct PostgresParams {
    pub connection_limit: u32,
    pub dbname: String,
    pub schema: String,
    pub config: postgres::Config,
}

type ConnectionParams = (Vec<(String, String)>, Vec<(String, String)>);

impl TryFrom<Url> for PostgresParams {
    type Error = Error;

    fn try_from(mut url: Url) -> crate::Result<Self> {
        let official = vec![];

        let (supported, unsupported): ConnectionParams = url
            .query_pairs()
            .map(|(k, v)| (String::from(k), String::from(v)))
            .collect::<Vec<(String, String)>>()
            .into_iter()
            .partition(|(k, _)| official.contains(&k.as_str()));

        url.query_pairs_mut().clear();

        supported.into_iter().for_each(|(k, v)| {
            url.query_pairs_mut().append_pair(&k, &v);
        });

        let mut config = postgres::Config::new();

        match percent_decode(url.username().as_bytes()).decode_utf8() {
            Ok(username) => {
                config.user(username.borrow());
            }
            Err(_) => {
                warn!("Couldn't decode username to UTF-8, using the non-decoded version.");
                config.user(url.username());
            }
        }

        match url
            .password()
            .and_then(|pw| percent_decode(pw.as_bytes()).decode_utf8().ok())
        {
            Some(password) => {
                let pw: &str = password.borrow();
                config.password(pw);
            }
            None => {
                config.password(url.password().unwrap_or(""));
            }
        }

        config.host(url.host_str().unwrap_or("localhost"));
        config.port(url.port().unwrap_or(5432));

        let dbname = match url.path_segments() {
            Some(mut segments) => segments.next().unwrap_or("postgres"),
            None => "postgres",
        };

        config.dbname(dbname);
        config.connect_timeout(Duration::from_millis(5000));

        let mut connection_limit = num_cpus::get_physical() * 2 + 1;
        let mut schema = String::from(DEFAULT_SCHEMA);

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
                "schema" => {
                    schema = v.to_string();
                }
                "connection_limit" => {
                    let as_int: usize = v.parse().map_err(|_| Error::InvalidConnectionArguments)?;
                    connection_limit = as_int;
                }
                _ => trace!("Discarding connection string param: {}", k),
            };
        }

        Ok(Self {
            connection_limit: u32::try_from(connection_limit).unwrap(),
            schema,
            config,
            dbname: dbname.to_string(),
        })
    }
}

impl TryFrom<Url> for PostgreSql {
    type Error = Error;

    fn try_from(url: Url) -> crate::Result<Self> {
        let params = PostgresParams::try_from(url)?;
        PostgreSql::new(params.config, Some(params.schema))
    }
}

impl From<postgres::Client> for PostgreSql {
    fn from(client: postgres::Client) -> Self {
        Self { client }
    }
}

impl PostgreSql {
    pub fn new(config: postgres::Config, schema: Option<String>) -> crate::Result<Self> {
        let mut tls_builder = TlsConnector::builder();
        tls_builder.danger_accept_invalid_certs(true); // For Heroku

        let tls = MakeTlsConnector::new(tls_builder.build()?);
        let schema = schema.unwrap_or_else(|| String::from(DEFAULT_SCHEMA));

        let mut client = metrics::connect("postgres", || config.connect(tls))?;
        client.execute(format!("SET search_path = \"{}\"", schema).as_str(), &[])?;

        Ok(Self::from(client))
    }

    pub fn from_params(params: PostgresParams) -> crate::Result<Self> {
        Self::new(params.config, Some(params.schema))
    }
}

impl Queryable for PostgreSql {
    fn execute<'a>(&mut self, q: Query<'a>) -> crate::Result<Option<Id>> {
        let (sql, params) = visitor::Postgres::build(q);

        metrics::query("postgres.execute", &sql, &params, || {
            let stmt = self.client.prepare(&sql)?;
            let rows = self
                .client
                .query(&stmt, &conversion::conv_params(&params))?;

            let id: Option<Id> = rows.into_iter().rev().next().map(|row| {
                let id: Id = row.get(0);
                id
            });

            Ok(id)
        })
    }

    fn query<'a>(&mut self, q: Query<'a>) -> crate::Result<ResultSet> {
        let (sql, params) = visitor::Postgres::build(q);
        self.query_raw(sql.as_str(), &params[..])
    }

    fn query_raw<'a>(
        &mut self,
        sql: &str,
        params: &[ParameterizedValue<'a>],
    ) -> crate::Result<ResultSet> {
        metrics::query("postgres.query_raw", sql, params, || {
            let stmt = self.client.prepare(sql)?;
            let rows = self.client.query(&stmt, &conversion::conv_params(params))?;

            let mut result = ResultSet::new(stmt.to_column_names(), Vec::new());

            for row in rows {
                result.rows.push(row.to_result_row()?);
            }

            Ok(result)
        })
    }

    fn execute_raw<'a>(
        &mut self,
        sql: &str,
        params: &[ParameterizedValue<'a>],
    ) -> crate::Result<u64> {
        metrics::query("postgres.execute_raw", sql, params, || {
            let stmt = self.client.prepare(sql)?;

            let changes = self
                .client
                .execute(&stmt, &conversion::conv_params(params))?;

            Ok(changes)
        })
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
        metrics::query("postgres.raw_cmd", cmd, &[], || {
            self.client.simple_query(cmd)?;
            Ok(())
        })
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
        let mut connection = PostgreSql::new(get_config(), None).unwrap();

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
        let mut connection = PostgreSql::new(get_config(), None).unwrap();
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
        let mut connection = PostgreSql::new(get_config(), None).unwrap();
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

    #[test]
    fn test_custom_search_path() {
        let conn_string = format!(
            "postgresql://{}:{}@{}:{}/{}?schema=musti-test",
            env::var("TEST_PG_USER").unwrap(),
            env::var("TEST_PG_PASSWORD").unwrap(),
            env::var("TEST_PG_HOST").unwrap(),
            env::var("TEST_PG_PORT").unwrap(),
            env::var("TEST_PG_DB").unwrap(),
        );

        let url = Url::parse(&conn_string).unwrap();
        let mut client = PostgreSql::try_from(url).unwrap();

        let result_set = client.query_raw("SHOW search_path", &[]).unwrap();
        let row = result_set.first().unwrap();

        assert_eq!(Some("\"musti-test\""), row[0].as_str());
    }

    #[test]
    fn should_map_nonexisting_database_error() {
        let mut config = get_config();
        config.dbname("this_does_not_exist");

        let res = PostgreSql::new(config, None);

        assert!(res.is_err());

        match res.unwrap_err() {
            Error::DatabaseDoesNotExist { db_name } => assert_eq!("this_does_not_exist", db_name.as_str()),
            e => panic!("Expected `DatabaseDoesNotExist`, got {:?}", e),
        }
    }

    #[test]
    fn should_map_authentication_failed_error() {
        let mut admin = PostgreSql::new(get_config(), None).unwrap();
        admin
            .execute_raw("CREATE USER should_map_access_denied_test with password 'password'", &[])
            .unwrap();

        let res = std::panic::catch_unwind(|| {
            let mut config = get_config();
            config.user("should_map_access_denied_test");
            config.password("catword");

            let conn = PostgreSql::new(config, None);

            assert!(conn.is_err());

            match conn.unwrap_err() {
                Error::AuthenticationFailed { user } => assert_eq!("should_map_access_denied_test", user.as_str()),
                e => panic!("Expected `AuthenticationFailed`, got {:?}", e),
            }
        });

        admin
            .execute_raw("DROP USER should_map_access_denied_test", &[])
            .unwrap();
        res.unwrap();
    }

    #[test]
    fn should_map_database_already_exists_error() {
        let mut admin = PostgreSql::new(get_config(), None).unwrap();

        admin
            .execute_raw("CREATE DATABASE should_map_if_database_already_exists", &[])
            .unwrap();

        let res = std::panic::catch_unwind(|| {
            let mut admin = PostgreSql::new(get_config(), None).unwrap();

            let res = admin
                .execute_raw("CREATE DATABASE should_map_if_database_already_exists", &[]);

            assert!(res.is_err());

            match res.unwrap_err() {
                Error::DatabaseAlreadyExists { db_name } => assert_eq!("should_map_if_database_already_exists", db_name.as_str()),
                e => panic!("Expected `DatabaseAlreadyExists`, got {:?}", e),
            }
        });

        admin
            .execute_raw("DROP DATABASE should_map_if_database_already_exists", &[])
            .unwrap();

        res.unwrap();
    }

    #[test]
    fn postgres_params_from_url_should_capture_database_name() {
        let url: Url = "postgresql://postgres:prisma@127.0.0.1:5432/pgress?schema=test_schema"
            .parse()
            .unwrap();
        let params = PostgresParams::try_from(url).unwrap();
        assert_eq!(params.dbname, "pgress");
        assert_eq!(params.schema, "test_schema");
    }
}
