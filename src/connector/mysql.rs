use crate::{
    ast::{Id, ParameterizedValue, Query},
    error::Error,
    transaction::{
        ColumnNames, Connection, Connectional, ResultRow, ToColumnNames, ToResultRow, Transaction,
        Transactional,
    },
    visitor::{self, Visitor},
    QueryResult, ResultSet,
};
use chrono::{DateTime, Duration, NaiveDate, Utc};
use mysql as my;
use r2d2_mysql::pool::MysqlConnectionManager;

type Pool = r2d2::Pool<MysqlConnectionManager>;
#[allow(unused)] // We implement a trait on the alias, it is used.
type PooledConnection = r2d2::PooledConnection<MysqlConnectionManager>;

/// The World's Most Advanced Open Source Relational Database
pub struct Mysql {
    pool: Pool,
}

impl Mysql {
    pub fn new(conf: mysql::OptsBuilder) -> QueryResult<Mysql> {
        let manager = MysqlConnectionManager::new(conf);

        Ok(Mysql {
            pool: r2d2::Pool::builder().build(manager)?,
        })
    }
}

impl Transactional for Mysql {
    fn with_transaction<F, T>(&self, _db: &str, f: F) -> QueryResult<T>
    where
        F: FnOnce(&mut Transaction) -> QueryResult<T>,
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
    fn with_connection<F, T>(&self, _db: &str, f: F) -> QueryResult<T>
    where
        F: FnOnce(&mut Connection) -> QueryResult<T>,
    {
        dbg!(self.pool.state());
        let mut conn = self.pool.get()?;
        let result = f(&mut conn);
        result
    }
}

fn conv_params(params: &[ParameterizedValue]) -> my::params::Params {
    if params.len() > 0 {
        my::params::Params::Positional(params.iter().map(|x| x.into()).collect::<Vec<my::Value>>())
    } else {
        // If we don't use explicit 'Empty',
        // mysql crashes with 'internal error: entered unreachable code'
        my::params::Params::Empty
    }
}

impl<'a> Transaction for my::Transaction<'a> {}

impl<'a> Connection for my::Transaction<'a> {
    fn execute(&mut self, q: Query) -> QueryResult<Option<Id>> {
        let (sql, params) = dbg!(visitor::Mysql::build(q));
        let mut stmt = self.prepare(&sql)?;
        let _rows = stmt.execute(conv_params(&params))?;

        // TODO: Return last inserted ID is not implemented for mysql.
        Ok(None)
    }

    fn query(&mut self, q: Query) -> QueryResult<ResultSet> {
        let (sql, params) = dbg!(visitor::Postgres::build(q));

        self.query_raw(&sql, &params[..])
    }

    fn query_raw(&mut self, sql: &str, params: &[ParameterizedValue]) -> QueryResult<ResultSet> {
        let mut stmt = self.prepare(&sql)?;
        let mut result = ResultSet::new(&stmt.to_column_names(), Vec::new());
        let rows = stmt.execute(conv_params(params))?;

        for row in rows {
            result.rows.push(row?.to_result_row()?);
        }

        Ok(result)
    }
}

impl Connection for PooledConnection {
    fn execute(&mut self, q: Query) -> QueryResult<Option<Id>> {
        let (sql, params) = dbg!(visitor::Mysql::build(q));
        let mut stmt = self.prepare(&sql)?;
        let _rows = stmt.execute(conv_params(&params))?;

        // TODO: Return last inserted ID is not implemented for mysql.
        Ok(None)
    }

    fn query(&mut self, q: Query) -> QueryResult<ResultSet> {
        let (sql, params) = dbg!(visitor::Postgres::build(q));

        self.query_raw(&sql, &params[..])
    }

    fn query_raw(&mut self, sql: &str, params: &[ParameterizedValue]) -> QueryResult<ResultSet> {
        let mut stmt = self.prepare(&sql)?;
        let mut result = ResultSet::new(&stmt.to_column_names(), Vec::new());
        let rows = stmt.execute(conv_params(params))?;

        for row in rows {
            result.rows.push(row?.to_result_row()?);
        }

        Ok(result)
    }
}

impl ToResultRow for my::Row {
    fn to_result_row<'b>(&'b self) -> QueryResult<ResultRow> {
        fn convert(row: &my::Row, i: usize) -> QueryResult<ParameterizedValue> {
            // TODO: It would prob. be better to inver via Column::column_type()
            let raw_value = row.as_ref(i).unwrap_or(&my::Value::NULL);
            let res = match raw_value {
                my::Value::NULL => ParameterizedValue::Null,
                my::Value::Bytes(b) => ParameterizedValue::Text(String::from_utf8(b.to_vec())?),
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

        let mut row = ResultRow::default();

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
        Error::QueryError(e.into())
    }
}

impl From<std::string::FromUtf8Error> for Error {
    fn from(e: std::string::FromUtf8Error) -> Error {
        Error::QueryError(e.into())
    }
}
