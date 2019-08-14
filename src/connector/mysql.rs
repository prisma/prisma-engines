mod conversion;
mod error;

use mysql as my;
use url::Url;
use percent_encoding::percent_decode;
use std::{convert::TryFrom, time::Duration};

use crate::{
    ast::{Id, ParameterizedValue, Query},
    error::Error,
    connector::{queryable::*, ResultSet, Transaction, metrics},
    visitor::{self, Visitor},
};

/// A connector interface for the MySQL database.
pub struct Mysql {
    pub(crate) client: my::Conn,
}

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
            },
            Err(_) => {
                warn!("Couldn't decode username to UTF-8, using the non-decoded version.");
                config.user(Some(url.username()));
            }
        }

        match url.password().and_then(|pw| percent_decode(pw.as_bytes()).decode_utf8().ok()) {
            Some(password) => {
                config.pass(Some(password));
            },
            None => {
                config.pass(url.password());
            }
        }

        config.ip_or_hostname(url.host_str());
        config.tcp_port(url.port().unwrap_or(3306));
        config.verify_peer(false);
        config.stmt_cache_size(Some(1000));
        config.tcp_connect_timeout(Some(Duration::from_millis(5000)));

        let dbname = match url.path_segments() {
            Some(mut segments) => {
                segments.next().unwrap_or("mysql")
            },
            None => {
                "mysql"
            },
        };

        config.db_name(Some(dbname));

        let mut connection_limit = 2;

        for (k, v) in unsupported.into_iter() {
            match k.as_ref() {
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
            dbname: dbname.to_string(),
        })
    }
}

impl TryFrom<Url> for Mysql {
    type Error = Error;

    fn try_from(url: Url) -> crate::Result<Self> {
        let params = MysqlParams::try_from(url)?;
        Mysql::new(params.config)
    }
}

impl From<my::Conn> for Mysql {
    fn from(client: my::Conn) -> Self {
        Self { client }
    }
}

impl Mysql {
    pub fn new(conf: my::OptsBuilder) -> crate::Result<Self> {
        let client = metrics::connect("mysql", || my::Conn::new(conf))?;
        Ok(Self::from(client))
    }
}

impl Queryable for Mysql {
    fn execute(&mut self, q: Query) -> crate::Result<Option<Id>> {
        metrics::query("mysql.execute", || {
            let (sql, params) = visitor::Mysql::build(q);

            let mut stmt = self.client.prepare(&sql)?;
            let result = stmt.execute(params)?;

            Ok(Some(Id::from(result.last_insert_id())))
        })
    }

    fn query<'a>(&mut self, q: Query<'a>) -> crate::Result<ResultSet> {
        metrics::query("mysql.query", || {
            let (sql, params) = visitor::Mysql::build(q);
            self.query_raw(&sql, &params[..])
        })
    }

    fn query_raw<'a>(
        &mut self,
        sql: &str,
        params: &[ParameterizedValue<'a>],
    ) -> crate::Result<ResultSet> {
        metrics::query("mysql.query_raw", || {
            let mut stmt = self.client.prepare(&sql)?;
            let mut result = ResultSet::new(stmt.to_column_names(), Vec::new());
            let rows = stmt.execute(conversion::conv_params(params))?;

            for row in rows {
                result.rows.push(row?.to_result_row()?);
            }

            Ok(result)
        })
    }

    fn execute_raw<'a>(
        &mut self,
        sql: &str,
        params: &[ParameterizedValue<'a>],
    ) -> crate::Result<u64> {
        metrics::query("mysql.execute_raw", || {
            let mut stmt = self.client.prepare(sql)?;
            let result = stmt.execute(conversion::conv_params(params))?;

            Ok(result.affected_rows())
        })
    }

    fn turn_off_fk_constraints(&mut self) -> crate::Result<()> {
        self.client.query("SET FOREIGN_KEY_CHECKS=0")?;
        Ok(())
    }

    fn turn_on_fk_constraints(&mut self) -> crate::Result<()> {
        self.client.query("SET FOREIGN_KEY_CHECKS=1")?;
        Ok(())
    }

    fn start_transaction<'b>(&'b mut self) -> crate::Result<Transaction<'b>> {
        Ok(Transaction::new(self)?)
    }

    fn raw_cmd(&mut self, cmd: &str) -> crate::Result<()> {
        self.client.query(cmd)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::connector::Queryable;
    use mysql::OptsBuilder;
    use std::env;

    fn get_config() -> OptsBuilder {
        let mut config = OptsBuilder::new();
        config.ip_or_hostname(env::var("TEST_MYSQL_HOST").ok());
        config.tcp_port(env::var("TEST_MYSQL_PORT").unwrap().parse::<u16>().unwrap());
        config.db_name(env::var("TEST_MYSQL_DB").ok());
        config.pass(env::var("TEST_MYSQL_PASSWORD").ok());
        config.user(env::var("TEST_MYSQL_USER").ok());
        config
    }

    #[test]
    fn should_provide_a_database_connection() {
        let mut connection = Mysql::new(get_config()).unwrap();

        let res = connection
            .query_raw(
                "select * from information_schema.`COLUMNS` where COLUMN_NAME = 'unknown_123'",
                &[],
            )
            .unwrap();

        assert!(res.is_empty());
    }

    #[test]
    fn should_provide_a_database_transaction() {
        let mut connection = Mysql::new(get_config()).unwrap();
        let mut tx = connection.start_transaction().unwrap();

        let res = tx
            .query_raw(
                "select * from information_schema.`COLUMNS` where COLUMN_NAME = 'unknown_123'",
                &[],
            )
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

    #[test]
    fn should_map_columns_correctly() {
        let mut connection = Mysql::new(get_config()).unwrap();

        connection.query_raw(DROP_TABLE, &[]).unwrap();
        connection.query_raw(TABLE_DEF, &[]).unwrap();
        connection.query_raw(CREATE_USER, &[]).unwrap();

        let rows = connection.query_raw("SELECT * FROM `user`", &[]).unwrap();
        assert_eq!(rows.len(), 1);

        let row = rows.get(0).unwrap();
        assert_eq!(row["id"].as_i64(), Some(1));
        assert_eq!(row["name"].as_str(), Some("Joe"));
        assert_eq!(row["age"].as_i64(), Some(27));
        assert_eq!(row["salary"].as_f64(), Some(20000.0));
    }
}
