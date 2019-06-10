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
use chrono::{DateTime, NaiveDateTime, Utc};
use native_tls::TlsConnector;
use postgres::{
    types::{FromSql, ToSql, Type as PostgresType},
    Client as PostgresConnection, Config, Statement as PostgresStatement,
    Transaction as PostgresTransaction,
};
use r2d2_postgres::PostgresConnectionManager;
use rust_decimal::Decimal;
use tokio_postgres::Row as PostgresRow;
use tokio_postgres_native_tls::MakeTlsConnector;
use uuid::Uuid;

type Pool = r2d2::Pool<PostgresConnectionManager<MakeTlsConnector>>;

/// The World's Most Advanced Open Source Relational Database
pub struct PostgreSql {
    pool: Pool,
}

impl<'a> FromSql<'a> for Id {
    fn from_sql(
        ty: &PostgresType,
        raw: &'a [u8],
    ) -> Result<Id, Box<dyn std::error::Error + Sync + Send>> {
        let res = match *ty {
            PostgresType::INT2 => Id::Int(i16::from_sql(ty, raw)? as usize),
            PostgresType::INT4 => Id::Int(i32::from_sql(ty, raw)? as usize),
            PostgresType::INT8 => Id::Int(i64::from_sql(ty, raw)? as usize),
            PostgresType::UUID => Id::UUID(Uuid::from_sql(ty, raw)?),
            _ => Id::String(String::from_sql(ty, raw)?),
        };

        Ok(res)
    }

    fn accepts(ty: &PostgresType) -> bool {
        <&str as FromSql>::accepts(ty)
            || <Uuid as FromSql>::accepts(ty)
            || <i16 as FromSql>::accepts(ty)
            || <i32 as FromSql>::accepts(ty)
            || <i64 as FromSql>::accepts(ty)
    }
}

impl Transactional for PostgreSql {
    fn with_transaction<F, T>(&self, _db: &str, f: F) -> QueryResult<T>
    where
        F: FnOnce(&mut Transaction) -> QueryResult<T>,
    {
        self.with_connection_internal(|client| {
            let mut tx = client.transaction()?;
            let result = f(&mut tx);

            if result.is_ok() {
                tx.commit()?;
            }

            result
        })
    }
}

impl Connectional for PostgreSql {
    fn with_connection<F, T>(&self, _db: &str, f: F) -> QueryResult<T>
    where
        F: FnOnce(&mut Connection) -> QueryResult<T>,
    {
        // TODO: Select DB.
        self.with_connection_internal(|mut client| {
            let result = f(&mut client);
            result
        })
    }
}

impl<'a> Transaction for PostgresTransaction<'a> {}

// Postgres uses a somewhat weird parameter format, therefore
// we have to re-map all elements.
fn conv_params(params: &[ParameterizedValue]) -> Vec<&tokio_postgres::types::ToSql> {
    params.iter().map(|x| x as &ToSql).collect::<Vec<_>>()
}

impl<'a> Connection for PostgresTransaction<'a> {
    fn execute(&mut self, q: Query) -> QueryResult<Option<Id>> {
        let (sql, params) = dbg!(visitor::Postgres::build(q));

        let stmt = self.prepare(&sql)?;
        let rows = PostgresTransaction::query(self, &stmt, &conv_params(&params))?;

        let id = rows.into_iter().rev().next().map(|row| {
            let id = row.get(0);
            let tpe = row.columns()[0].type_();
            Id::from_sql(tpe, id)
        });

        match id {
            Some(Ok(id)) => Ok(Some(id)),
            Some(Err(_)) => panic!("Cannot convert err, todo."),
            None => Ok(None),
        }
    }

    fn query(&mut self, q: Query) -> QueryResult<ResultSet> {
        let (sql, params) = dbg!(visitor::Postgres::build(q));

        self.query_raw(&sql, &params[..])
    }

    fn query_raw(&mut self, sql: &str, params: &[ParameterizedValue]) -> QueryResult<ResultSet> {
        let stmt = self.prepare(&sql)?;
        let rows = PostgresTransaction::query(self, &stmt, &conv_params(params))?;

        let mut result = ResultSet::new(&stmt.to_column_names(), Vec::new());

        for row in rows {
            result.rows.push(row.to_result_row()?);
        }

        Ok(result)
    }
}

impl Connection for &mut PostgresConnection {
    fn execute(&mut self, q: Query) -> QueryResult<Option<Id>> {
        let (sql, params) = dbg!(visitor::Postgres::build(q));

        let stmt = self.prepare(&sql)?;
        let rows = PostgresConnection::query(self, &stmt, &conv_params(&params))?;

        let id = rows.into_iter().rev().next().map(|row| {
            let id = row.get(0);
            let tpe = row.columns()[0].type_();
            Id::from_sql(tpe, id)
        });

        match id {
            Some(Ok(id)) => Ok(Some(id)),
            Some(Err(_)) => panic!("Cannot convert err, todo."),
            None => Ok(None),
        }
    }

    fn query(&mut self, q: Query) -> QueryResult<ResultSet> {
        let (sql, params) = dbg!(visitor::Postgres::build(q));

        self.query_raw(&sql, &params)
    }

    fn query_raw(&mut self, sql: &str, params: &[ParameterizedValue]) -> QueryResult<ResultSet> {
        let stmt = self.prepare(&sql)?;
        let rows = PostgresConnection::query(self, &stmt, &conv_params(params))?;

        let mut result = ResultSet::new(&stmt.to_column_names(), Vec::new());

        for row in rows {
            result.rows.push(row.to_result_row()?);
        }

        Ok(result)
    }
}

impl ToResultRow for PostgresRow {
    fn to_result_row<'b>(&'b self) -> QueryResult<ResultRow> {
        fn convert(row: &PostgresRow, i: usize) -> QueryResult<ParameterizedValue> {
            let result = match *row.columns()[i].type_() {
                PostgresType::BOOL => match row.try_get(i)? {
                    Some(val) => ParameterizedValue::Boolean(val),
                    None => ParameterizedValue::Null,
                },
                PostgresType::INT2 => match row.try_get(i)? {
                    Some(val) => {
                        let val: i16 = val;
                        ParameterizedValue::Integer(val as i64)
                    }
                    None => ParameterizedValue::Null,
                },
                PostgresType::INT4 => match row.try_get(i)? {
                    Some(val) => {
                        let val: i32 = val;
                        ParameterizedValue::Integer(val as i64)
                    }
                    None => ParameterizedValue::Null,
                },
                PostgresType::INT8 => match row.try_get(i)? {
                    Some(val) => {
                        let val: i64 = val;
                        ParameterizedValue::Integer(val)
                    }
                    None => ParameterizedValue::Null,
                },
                PostgresType::NUMERIC => match row.try_get(i)? {
                    Some(val) => {
                        let val: Decimal = val;
                        let val: f64 = val.to_string().parse().unwrap();
                        ParameterizedValue::Real(val)
                    }
                    None => ParameterizedValue::Null,
                },
                PostgresType::FLOAT4 => match row.try_get(i)? {
                    Some(val) => {
                        let val: f32 = val;
                        ParameterizedValue::Real(val as f64)
                    }
                    None => ParameterizedValue::Null,
                },
                PostgresType::FLOAT8 => match row.try_get(i)? {
                    Some(val) => {
                        let val: f64 = val;
                        ParameterizedValue::Real(val)
                    }
                    None => ParameterizedValue::Null,
                },
                PostgresType::TIMESTAMP => match row.try_get(i)? {
                    Some(val) => {
                        let ts: NaiveDateTime = val;
                        let dt = DateTime::<Utc>::from_utc(ts, Utc);
                        ParameterizedValue::DateTime(dt)
                    }
                    None => ParameterizedValue::Null,
                },
                PostgresType::UUID => match row.try_get(i)? {
                    Some(val) => {
                        let val: Uuid = val;
                        ParameterizedValue::Uuid(val)
                    }
                    None => ParameterizedValue::Null,
                },
                #[cfg(feature = "array")]
                PostgresType::INT2_ARRAY => match row.try_get(i)? {
                    Some(val) => {
                        let val: Vec<i16> = val;
                        ParameterizedValue::Array(
                            val.into_iter()
                                .map(|x| ParameterizedValue::Integer(x as i64))
                                .collect(),
                        )
                    }
                    None => ParameterizedValue::Null,
                },
                #[cfg(feature = "array")]
                PostgresType::INT4_ARRAY => match row.try_get(i)? {
                    Some(val) => {
                        let val: Vec<i32> = val;
                        ParameterizedValue::Array(
                            val.into_iter()
                                .map(|x| ParameterizedValue::Integer(x as i64))
                                .collect(),
                        )
                    }
                    None => ParameterizedValue::Null,
                },
                #[cfg(feature = "array")]
                PostgresType::INT8_ARRAY => match row.try_get(i)? {
                    Some(val) => {
                        let val: Vec<i64> = val;
                        ParameterizedValue::Array(
                            val.into_iter()
                                .map(|x| ParameterizedValue::Integer(x as i64))
                                .collect(),
                        )
                    }
                    None => ParameterizedValue::Null,
                },
                #[cfg(feature = "array")]
                PostgresType::FLOAT4_ARRAY => match row.try_get(i)? {
                    Some(val) => {
                        let val: Vec<f32> = val;
                        ParameterizedValue::Array(
                            val.into_iter()
                                .map(|x| ParameterizedValue::Real(x as f64))
                                .collect(),
                        )
                    }
                    None => ParameterizedValue::Null,
                },
                #[cfg(feature = "array")]
                PostgresType::FLOAT8_ARRAY => match row.try_get(i)? {
                    Some(val) => {
                        let val: Vec<f64> = val;
                        ParameterizedValue::Array(
                            val.into_iter()
                                .map(|x| ParameterizedValue::Real(x as f64))
                                .collect(),
                        )
                    }
                    None => ParameterizedValue::Null,
                },
                #[cfg(feature = "array")]
                PostgresType::BOOL_ARRAY => match row.try_get(i)? {
                    Some(val) => {
                        let val: Vec<bool> = val;
                        ParameterizedValue::Array(
                            val.into_iter()
                                .map(|x| ParameterizedValue::Boolean(x))
                                .collect(),
                        )
                    }
                    None => ParameterizedValue::Null,
                },
                #[cfg(feature = "array")]
                PostgresType::TIMESTAMP_ARRAY => match row.try_get(i)? {
                    Some(val) => {
                        let val: Vec<NaiveDateTime> = val;
                        ParameterizedValue::Array(
                            val.into_iter()
                                .map(|x| {
                                    ParameterizedValue::DateTime(DateTime::<Utc>::from_utc(x, Utc))
                                })
                                .collect(),
                        )
                    }
                    None => ParameterizedValue::Null,
                },
                #[cfg(feature = "array")]
                PostgresType::NUMERIC_ARRAY => match row.try_get(i)? {
                    Some(val) => {
                        let val: Vec<Decimal> = val;
                        ParameterizedValue::Array(
                            val.into_iter()
                                .map(|x| ParameterizedValue::Real(x.to_string().parse().unwrap()))
                                .collect(),
                        )
                    }
                    None => ParameterizedValue::Null,
                },
                #[cfg(feature = "array")]
                PostgresType::TEXT_ARRAY
                | PostgresType::NAME_ARRAY
                | PostgresType::VARCHAR_ARRAY => match row.try_get(i)? {
                    Some(val) => {
                        let val: Vec<&str> = val;
                        ParameterizedValue::Array(
                            val.into_iter()
                                .map(|x| ParameterizedValue::Text(String::from(x)))
                                .collect(),
                        )
                    }
                    None => ParameterizedValue::Null,
                },
                _ => match row.try_get(i)? {
                    Some(val) => ParameterizedValue::Text(val),
                    None => ParameterizedValue::Null,
                },
            };

            Ok(result)
        }

        let mut row = ResultRow::default();

        for i in 0..self.columns().len() {
            row.values.push(convert(self, i)?);
        }

        Ok(row)
    }
}

impl ToColumnNames for PostgresStatement {
    fn to_column_names<'b>(&'b self) -> ColumnNames {
        let mut names = ColumnNames::default();

        for column in self.columns() {
            names.names.push(String::from(column.name()));
        }

        names
    }
}

impl PostgreSql {
    pub fn new(config: Config, connections: u32) -> QueryResult<PostgreSql> {
        let mut tls_builder = TlsConnector::builder();
        tls_builder.danger_accept_invalid_certs(true); // For Heroku
        let tls = MakeTlsConnector::new(tls_builder.build()?);

        let manager = PostgresConnectionManager::new(config, tls);
        let pool = r2d2::Pool::builder().max_size(connections).build(manager)?;

        Ok(PostgreSql { pool })
    }

    fn with_connection_internal<F, T>(&self, f: F) -> QueryResult<T>
    where
        F: FnOnce(&mut PostgresConnection) -> QueryResult<T>,
    {
        let mut client = self.pool.get()?;
        let result = f(&mut client);
        result
    }
}

impl From<postgres::error::Error> for Error {
    fn from(e: postgres::error::Error) -> Error {
        dbg!(&e);
        // TODO: Ask J how to map to Failure::Error
        Error::NotFound
    }
}

impl From<native_tls::Error> for Error {
    fn from(e: native_tls::Error) -> Error {
        dbg!(&e);
        // TODO: Ask J how to map to Failure::Error
        Error::NotFound
    }
}
