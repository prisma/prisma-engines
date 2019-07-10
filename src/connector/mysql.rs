use crate::{
    ast::{Id, ParameterizedValue, Query},
    connector::{
        transaction::{
            ColumnNames, Connection, Connectional, Row, ToColumnNames, ToRow, Transaction,
            Transactional,
        },
        ResultSet,
    },
    error::Error,
    visitor::{self, Visitor},
};
use chrono::{DateTime, Duration, NaiveDate, Utc};
use mysql as my;
use r2d2_mysql::pool::MysqlConnectionManager;
use url::Url;

type Pool = r2d2::Pool<MysqlConnectionManager>;
#[allow(unused)] // We implement a trait on the alias, it is used.
type PooledConnection = r2d2::PooledConnection<MysqlConnectionManager>;

/// A connector interface for the MySQL database.
pub struct Mysql {
    pool: Pool,
    pub db_name: Option<String>,
}

impl Mysql {
    // TODO: we should not use this constructor since it does set the db_name field
    pub fn new(conf: mysql::OptsBuilder) -> crate::Result<Mysql> {
        let manager = MysqlConnectionManager::new(conf);

        Ok(Mysql {
            pool: r2d2::Pool::builder().build(manager)?,
            db_name: None,
        })
    }

    pub fn new_from_url(url: &str) -> crate::Result<Mysql> {
        // TODO: connection limit configuration
        let mut builder = my::OptsBuilder::new();
        let url = Url::parse(url)?;
        let db_name = url.path_segments().and_then(|mut segments| segments.next());

        builder.ip_or_hostname(url.host_str());
        builder.tcp_port(url.port().unwrap_or(3306));
        builder.user(Some(url.username()));
        builder.pass(url.password());
        builder.db_name(db_name);
        builder.verify_peer(false);
        builder.stmt_cache_size(Some(1000));

        let manager = MysqlConnectionManager::new(builder);

        Ok(Mysql {
            pool: r2d2::Pool::builder().build(manager)?,
            db_name: db_name.map(|x| x.to_string()),
        })
    }
}

impl Transactional for Mysql {
    type Error = Error;

    fn with_transaction<F, T>(&self, _db: &str, f: F) -> crate::Result<T>
    where
        F: FnOnce(&mut Transaction) -> crate::Result<T>,
    {
        let mut conn = self.pool.get()?;
        let mut tx = conn.start_transaction(true, None, None)?;
        let result = f(&mut tx);

        if result.is_ok() {
            tx.commit()?;
        }

        result
    }
}

impl Connectional for Mysql {
    fn with_connection<F, T>(&self, _db: &str, f: F) -> crate::Result<T>
    where
        F: FnOnce(&mut Connection) -> crate::Result<T>,
        Self: Sized,
    {
        let mut conn = self.pool.get()?;
        let result = f(&mut conn);
        result
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

fn conv_params<'a>(params: &[ParameterizedValue<'a>]) -> my::params::Params {
    if params.len() > 0 {
        my::params::Params::Positional(params.iter().map(|x| x.into()).collect::<Vec<my::Value>>())
    } else {
        // If we don't use explicit 'Empty',
        // mysql crashes with 'internal error: entered unreachable code'
        my::params::Params::Empty
    }
}

impl<'a> Transaction for my::Transaction<'a> {}

impl<'t> Connection for my::Transaction<'t> {
    fn execute<'a>(&mut self, q: Query<'a>) -> crate::Result<Option<Id>> {
        let (sql, params) = dbg!(visitor::Mysql::build(q));
        let mut stmt = self.prepare(&sql)?;
        let _rows = stmt.execute(conv_params(&params))?;

        // TODO: Return last inserted ID is not implemented for mysql.
        Ok(None)
    }

    fn query<'a>(&mut self, q: Query<'a>) -> crate::Result<ResultSet> {
        let (sql, params) = dbg!(visitor::Mysql::build(q));

        self.query_raw(&sql, &params[..])
    }

    fn query_raw<'a>(
        &mut self,
        sql: &str,
        params: &[ParameterizedValue<'a>],
    ) -> crate::Result<ResultSet> {
        let mut stmt = self.prepare(&sql)?;
        let mut result = ResultSet::new(stmt.to_column_names(), Vec::new());
        let rows = stmt.execute(conv_params(params))?;

        for row in rows {
            result.rows.push(row?.to_result_row()?);
        }

        Ok(result)
    }
}

impl Connection for PooledConnection {
    fn execute<'a>(&mut self, q: Query<'a>) -> crate::Result<Option<Id>> {
        let (sql, params) = dbg!(visitor::Mysql::build(q));
        let mut stmt = self.prepare(&sql)?;
        let _rows = stmt.execute(conv_params(&params))?;

        Ok(Some(Id::Int(_rows.last_insert_id() as usize)))
    }

    fn query<'a>(&mut self, q: Query<'a>) -> crate::Result<ResultSet> {
        let (sql, params) = dbg!(visitor::Mysql::build(q));

        self.query_raw(&sql, &params[..])
    }

    fn query_raw<'a>(
        &mut self,
        sql: &str,
        params: &[ParameterizedValue<'a>],
    ) -> crate::Result<ResultSet> {
        let mut stmt = self.prepare(&sql)?;
        let mut result = ResultSet::new(stmt.to_column_names(), Vec::new());
        let rows = stmt.execute(conv_params(params))?;

        for row in rows {
            result.rows.push(row?.to_result_row()?);
        }

        Ok(result)
    }
}

impl ToRow for my::Row {
    fn to_result_row<'b>(&'b self) -> crate::Result<Row> {
        fn convert(row: &my::Row, i: usize) -> crate::Result<ParameterizedValue<'static>> {
            // TODO: It would prob. be better to inver via Column::column_type()
            let raw_value = row.as_ref(i).unwrap_or(&my::Value::NULL);
            let res = match raw_value {
                my::Value::NULL => ParameterizedValue::Null,
                my::Value::Bytes(b) => {
                    ParameterizedValue::Text(String::from_utf8(b.to_vec())?.into())
                }
                my::Value::Int(i) => ParameterizedValue::Integer(*i),
                // TOOD: This is unsafe
                my::Value::UInt(i) => ParameterizedValue::Integer(*i as i64),
                my::Value::Float(f) => ParameterizedValue::Real(*f),
                my::Value::Date(year, month, day, hour, min, sec, _) => {
                    let naive = NaiveDate::from_ymd(*year as i32, *month as u32, *day as u32)
                        .and_hms(*hour as u32, *min as u32, *sec as u32);

                    let dt: DateTime<Utc> = DateTime::from_utc(naive, Utc);
                    ParameterizedValue::DateTime(dt)
                }
                my::Value::Time(is_neg, days, hours, minutes, seconds, micros) => {
                    let days = Duration::days(*days as i64);
                    let hours = Duration::hours(*hours as i64);
                    let minutes = Duration::minutes(*minutes as i64);
                    let seconds = Duration::seconds(*seconds as i64);
                    let micros = Duration::microseconds(*micros as i64);

                    let time = days
                        .checked_add(&hours)
                        .and_then(|t| t.checked_add(&minutes))
                        .and_then(|t| t.checked_add(&seconds))
                        .and_then(|t| t.checked_add(&micros))
                        .unwrap();

                    let duration = time.to_std().unwrap();
                    let f_time = duration.as_secs() as f64 + duration.subsec_micros() as f64 * 1e-6;

                    ParameterizedValue::Real(if *is_neg { -f_time } else { f_time })
                }
            };

            Ok(res)
        }

        let mut row = Row::default();

        for i in 0..self.len() {
            row.values.push(convert(self, i)?);
        }

        Ok(row)
    }
}

impl<'a> ToColumnNames for my::Stmt<'a> {
    fn to_column_names<'b>(&'b self) -> ColumnNames {
        let mut names = ColumnNames::default();

        if let Some(columns) = self.columns_ref() {
            for column in columns {
                names.names.push(String::from(column.name_str()));
            }
        }

        names
    }
}

impl From<my::error::Error> for Error {
    fn from(e: my::error::Error) -> Error {
        use my::error::MySqlError;

        match e {
            my::error::Error::MySqlError(MySqlError {
                state: _,
                ref message,
                code,
            }) if code == 1062 => {
                let splitted: Vec<&str> = message.split_whitespace().collect();
                let splitted: Vec<&str> = splitted.last().map(|s| s.split("'").collect()).unwrap();
                let splitted: Vec<&str> = splitted[1].split("_").collect();

                let field_name: String = splitted[0].into();

                Error::UniqueConstraintViolation { field_name }
            }
            my::error::Error::MySqlError(MySqlError {
                state: _,
                ref message,
                code,
            }) if code == 1263 => {
                let splitted: Vec<&str> = message.split_whitespace().collect();
                let splitted: Vec<&str> = splitted.last().map(|s| s.split("'").collect()).unwrap();
                let splitted: Vec<&str> = splitted[1].split("_").collect();

                let field_name: String = splitted[0].into();

                Error::NullConstraintViolation { field_name }
            }
            e => Error::QueryError(e.into()),
        }
    }
}

impl From<std::string::FromUtf8Error> for Error {
    fn from(e: std::string::FromUtf8Error) -> Error {
        Error::QueryError(e.into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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
        let connector = Mysql::new(get_config()).unwrap();

        connector
            .with_connection("TEST", |connection| {
                let res = connection.query_raw(
                    "select * from information_schema.`COLUMNS` where COLUMN_NAME = 'unknown_123'",
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
        let connector = Mysql::new(get_config()).unwrap();

        connector
            .with_transaction("TEST", |transaction| {
                let res = transaction.query_raw(
                    "select * from information_schema.`COLUMNS` where COLUMN_NAME = 'unknown_123'",
                    &[],
                )?;

                // No results expected.
                assert!(res.is_empty());

                Ok(())
            })
            .unwrap()
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
        let connector = Mysql::new(get_config()).unwrap();

        connector
            .with_connection("TEST", |connection| {
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

                Ok(())
            })
            .unwrap()
    }

}
