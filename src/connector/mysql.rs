mod conversion;
mod error;

use mysql_async::{self as my, prelude::Queryable as _};
use percent_encoding::percent_decode;
use std::{convert::TryFrom, path::Path};
use url::Url;

use crate::{
    ast::{Id, ParameterizedValue, Query},
    connector::{metrics, queryable::*, ResultSet, Transaction, DBIO},
    error::Error,
    visitor::{self, Visitor},
};

/// A connector interface for the MySQL database.
#[derive(Debug)]
pub struct Mysql {
    pub(crate) pool: my::Pool,
}

#[derive(Debug)]
pub struct MysqlParams {
    pub connection_limit: u32,
    pub dbname: String,
    pub config: my::OptsBuilder,
}

type ConnectionParams = (Vec<(String, String)>, Vec<(String, String)>);

impl TryFrom<Url> for MysqlParams {
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

        let mut config = my::OptsBuilder::new();

        match percent_decode(url.username().as_bytes()).decode_utf8() {
            Ok(username) => {
                config.user(Some(username.into_owned()));
            }
            Err(_) => {
                #[cfg(not(feature = "tracing-log"))]
                warn!("Couldn't decode username to UTF-8, using the non-decoded version.");
                #[cfg(feature = "tracing-log")]
                tracing::warn!("Couldn't decode username to UTF-8, using the non-decoded version.");

                config.user(Some(url.username()));
            }
        }

        match url
            .password()
            .and_then(|pw| percent_decode(pw.as_bytes()).decode_utf8().ok())
        {
            Some(password) => {
                config.pass(Some(password));
            }
            None => {
                config.pass(url.password());
            }
        }

        let mut connection_limit = num_cpus::get_physical() * 2 + 1;
        let mut ssl_opts = my::SslOpts::default();
        let mut use_ssl = false;

        for (k, v) in unsupported.into_iter() {
            match k.as_ref() {
                "connection_limit" => {
                    let as_int: usize = v.parse().map_err(|_| Error::InvalidConnectionArguments)?;
                    connection_limit = as_int;
                }
                "sslcert" => {
                    use_ssl = true;
                    ssl_opts.set_root_cert_path(Some(Path::new(&v).to_path_buf()));
                }
                "sslidentity" => {
                    use_ssl = true;
                    ssl_opts.set_pkcs12_path(Some(Path::new(&v).to_path_buf()));
                }
                "sslpassword" => {
                    use_ssl = true;
                    ssl_opts.set_password(Some(v.to_string()));
                }
                "sslaccept" => {
                    match v.as_ref() {
                        "strict" => {}
                        "accept_invalid_certs" => {
                            ssl_opts.set_danger_accept_invalid_certs(true);
                        }
                        _ => {
                            #[cfg(not(feature = "tracing-log"))]
                            debug!("Unsupported SSL accept mode {}, defaulting to `strict`", v);
                            #[cfg(feature = "tracing-log")]
                            tracing::debug!(
                                message = "Unsupported SSL accept mode, defaulting to `strict`",
                                mode = v.as_str()
                            );
                        }
                    };
                }
                _ => {
                    #[cfg(not(feature = "tracing-log"))]
                    trace!("Discarding connection string param: {}", k);
                    #[cfg(feature = "tracing-log")]
                    tracing::trace!(
                        message = "Discarding connection string param",
                        param = k.as_str()
                    );
                }
            };
        }

        let dbname = match url.path_segments() {
            Some(mut segments) => segments.next().unwrap_or("mysql"),
            None => "mysql",
        };

        config.db_name(Some(dbname));
        config.ip_or_hostname(url.host_str().unwrap_or("localhost"));
        config.tcp_port(url.port().unwrap_or(3306));
        config.stmt_cache_size(Some(1000));
        config.conn_ttl(Some(5000u32));

        if use_ssl {
            config.ssl_opts(Some(ssl_opts));
        }

        Ok(Self {
            connection_limit: u32::try_from(connection_limit).unwrap(),
            config,
            dbname: dbname.to_string(),
        })
    }
}

impl Mysql {
    pub fn new(mut opts: my::OptsBuilder) -> crate::Result<Self> {
        opts.pool_constraints(my::PoolConstraints::new(1, 1));

        Ok(Self {
            pool: my::Pool::new(opts),
        })
    }

    pub fn from_params(params: MysqlParams) -> crate::Result<Self> {
        Self::new(params.config)
    }

    fn execute_and_get_id<'a>(
        &'a self,
        sql: &'a str,
        params: &'a [ParameterizedValue],
    ) -> DBIO<'a, Option<Id>> {
        metrics::query("mysql.execute", sql, params, move || {
            async move {
                let conn = self.pool.get_conn().await?;
                let results = conn.prep_exec(sql, params.to_vec()).await?;

                Ok(results.last_insert_id().map(Id::from))
            }
        })
    }
}

impl Queryable for Mysql {
    fn execute<'a>(&'a self, q: Query<'a>) -> DBIO<'a, Option<Id>> {
        DBIO::new(async move {
            let (sql, params) = visitor::Mysql::build(q);
            self.execute_and_get_id(&sql, &params).await
        })
    }

    fn query<'a>(&'a self, q: Query<'a>) -> DBIO<'a, ResultSet> {
        DBIO::new(async move {
            let (sql, params) = visitor::Mysql::build(q);
            self.query_raw(&sql, &params).await
        })
    }

    fn query_raw<'a>(
        &'a self,
        sql: &'a str,
        params: &'a [ParameterizedValue],
    ) -> DBIO<'a, ResultSet> {
        metrics::query("mysql.query_raw", sql, params, move || {
            async move {
                let conn = self.pool.get_conn().await?;
                let results = conn.prep_exec(sql, conversion::conv_params(params)).await?;
                let columns = results
                    .columns_ref()
                    .iter()
                    .map(|s| s.name_str().into_owned())
                    .collect();

                let mut result_set = ResultSet::new(columns, Vec::new());
                let (_, rows) = results.map_and_drop(|row| row.to_result_row()).await?;

                for row in rows.into_iter() {
                    result_set.rows.push(row?);
                }

                Ok(result_set)
            }
        })
    }

    fn execute_raw<'a>(&'a self, sql: &'a str, params: &'a [ParameterizedValue]) -> DBIO<'a, u64> {
        metrics::query("mysql.execute_raw", sql, params, move || {
            async move {
                let conn = self.pool.get_conn().await?;
                let result = conn.prep_exec(sql, conversion::conv_params(params)).await?;

                Ok(result.affected_rows())
            }
        })
    }

    fn turn_off_fk_constraints(&self) -> DBIO<()> {
        DBIO::new(async move {
            let conn = self.pool.get_conn().await?;
            conn.query("SET FOREIGN_KEY_CHECKS=0").await?;

            Ok(())
        })
    }

    fn turn_on_fk_constraints(&self) -> DBIO<()> {
        DBIO::new(async move {
            let conn = self.pool.get_conn().await?;
            conn.query("SET FOREIGN_KEY_CHECKS=1").await?;

            Ok(())
        })
    }

    fn start_transaction(&self) -> DBIO<Transaction> {
        DBIO::new(async move { Transaction::new(self).await })
    }

    fn raw_cmd<'a>(&'a self, cmd: &'a str) -> DBIO<'a, ()> {
        metrics::query("mysql.raw_cmd", cmd, &[], move || {
            async move {
                let conn = self.pool.get_conn().await?;
                conn.query(cmd).await?;
                Ok(())
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::connector::Queryable;
    use mysql_async::OptsBuilder;
    use std::env;

    fn get_config() -> OptsBuilder {
        let mut config = OptsBuilder::new();
        config.ip_or_hostname(env::var("TEST_MYSQL_HOST").unwrap());
        config.tcp_port(env::var("TEST_MYSQL_PORT").unwrap().parse::<u16>().unwrap());
        config.db_name(env::var("TEST_MYSQL_DB").ok());
        config.pass(env::var("TEST_MYSQL_PASSWORD").ok());
        config.user(env::var("TEST_MYSQL_USER").ok());
        config
    }

    fn get_admin_config() -> OptsBuilder {
        let mut config = OptsBuilder::new();
        config.ip_or_hostname(env::var("TEST_MYSQL_HOST").unwrap());
        config.tcp_port(env::var("TEST_MYSQL_PORT").unwrap().parse::<u16>().unwrap());
        config.db_name(env::var("TEST_MYSQL_DB").ok());
        config.pass(env::var("TEST_MYSQL_ROOT_PASSWORD").ok());
        config.user("root".into());
        config
    }

    #[tokio::test]
    async fn should_provide_a_database_connection() {
        let connection = Mysql::new(get_config()).unwrap();

        let res = connection
            .query_raw(
                "select * from information_schema.`COLUMNS` where COLUMN_NAME = 'unknown_123'",
                &[],
            )
            .await
            .unwrap();

        assert!(res.is_empty());
    }

    #[tokio::test]
    async fn should_provide_a_database_transaction() {
        let connection = Mysql::new(get_config()).unwrap();
        let tx = connection.start_transaction().await.unwrap();

        let res = tx
            .query_raw(
                "select * from information_schema.`COLUMNS` where COLUMN_NAME = 'unknown_123'",
                &[],
            )
            .await
            .unwrap();

        assert!(res.is_empty());
    }

    const TABLE_DEF: &str = r#"
CREATE TABLE `user`(
    id       int4    PRIMARY KEY     NOT NULL,
    name     text    NOT NULL,
    age      int4    NOT NULL,
    salary   float4
);
"#;

    const CREATE_USER: &str = r#"
INSERT INTO `user` (id, name, age, salary)
VALUES (1, 'Joe', 27, 20000.00 );
"#;

    const DROP_TABLE: &str = "DROP TABLE IF EXISTS `user`;";

    #[tokio::test]
    async fn should_map_columns_correctly() {
        let connection = Mysql::new(get_config()).unwrap();

        connection.query_raw(DROP_TABLE, &[]).await.unwrap();
        connection.query_raw(TABLE_DEF, &[]).await.unwrap();
        connection.query_raw(CREATE_USER, &[]).await.unwrap();

        let rows = connection
            .query_raw("SELECT * FROM `user`", &[])
            .await
            .unwrap();
        assert_eq!(rows.len(), 1);

        let row = rows.get(0).unwrap();
        assert_eq!(row["id"].as_i64(), Some(1));
        assert_eq!(row["name"].as_str(), Some("Joe"));
        assert_eq!(row["age"].as_i64(), Some(27));
        assert_eq!(row["salary"].as_f64(), Some(20000.0));
    }

    #[tokio::test]
    async fn should_map_nonexisting_database_error() {
        let mut config = get_admin_config();
        config.db_name(Some("this_does_not_exist"));

        let conn = Mysql::new(config).unwrap();
        let res = conn.query_raw("SELECT 1 + 1", &[]).await;

        assert!(&res.is_err());

        match res.unwrap_err() {
            Error::DatabaseDoesNotExist { db_name } => {
                assert_eq!("this_does_not_exist", db_name.as_str())
            }
            e => panic!("Expected `DatabaseDoesNotExist`, got {:?}", e),
        }
    }
}
