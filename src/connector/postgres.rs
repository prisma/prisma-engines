use crate::{
    ast::{Id, ParameterizedValue, Query},
    error::Error,
    transaction::{
        ColumnNames, Connection, Connectional, ResultRow, ToColumnNames, ToResultRow, Transaction,
        Transactional,
    },
    visitor::{self, Visitor},
    ResultSet,
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
            _ => Id::String(String::from_sql(ty, raw)?.into()),
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
    type Error = Error;

    fn with_transaction<F, T>(&self, _db: &str, f: F) -> crate::Result<T>
    where
        F: FnOnce(&mut Transaction) -> crate::Result<T>,
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
    fn with_connection<F, T>(&self, _db: &str, f: F) -> crate::Result<T>
    where
        F: FnOnce(&mut Connection) -> crate::Result<T>,
        Self: Sized,
    {
        // TODO: Select DB.
        self.with_connection_internal(|mut client| {
            let result = f(&mut client);
            result
        })
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

impl<'a> Transaction for PostgresTransaction<'a> {}

// Postgres uses a somewhat weird parameter format, therefore
// we have to re-map all elements.
fn conv_params<'a>(params: &'a [ParameterizedValue<'a>]) -> Vec<&'a tokio_postgres::types::ToSql> {
    params.into_iter().map(|x| x as &ToSql).collect::<Vec<_>>()
}

impl<'t> Connection for PostgresTransaction<'t> {
    fn execute<'a>(&mut self, q: Query<'a>) -> crate::Result<Option<Id>> {
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

    fn query<'a>(&mut self, q: Query<'a>) -> crate::Result<ResultSet> {
        let (sql, params) = dbg!(visitor::Postgres::build(q));

        self.query_raw(&sql, &params[..])
    }

    fn query_raw<'a>(
        &mut self,
        sql: &str,
        params: &[ParameterizedValue<'a>],
    ) -> crate::Result<ResultSet> {
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
    fn execute<'a>(&mut self, q: Query<'a>) -> crate::Result<Option<Id>> {
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

    fn query<'a>(&mut self, q: Query<'a>) -> crate::Result<ResultSet> {
        let (sql, params) = dbg!(visitor::Postgres::build(q));

        self.query_raw(&sql, &params)
    }

    fn query_raw<'a>(
        &mut self,
        sql: &str,
        params: &[ParameterizedValue<'a>],
    ) -> crate::Result<ResultSet> {
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
    fn to_result_row<'b>(&'b self) -> crate::Result<ResultRow> {
        fn convert(row: &PostgresRow, i: usize) -> crate::Result<ParameterizedValue<'static>> {
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
                                .map(|x| ParameterizedValue::Text(String::from(x).into()))
                                .collect(),
                        )
                    }
                    None => ParameterizedValue::Null,
                },
                _ => match row.try_get(i)? {
                    Some(val) => {
                        let val: String = val;
                        ParameterizedValue::Text(val.into())
                    }
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
    pub fn new(config: Config, connections: u32) -> crate::Result<PostgreSql> {
        let mut tls_builder = TlsConnector::builder();
        tls_builder.danger_accept_invalid_certs(true); // For Heroku
        let tls = MakeTlsConnector::new(tls_builder.build()?);

        let manager = PostgresConnectionManager::new(config, tls);
        let pool = r2d2::Pool::builder().max_size(connections).build(manager)?;

        Ok(PostgreSql { pool })
    }

    fn with_connection_internal<F, T>(&self, f: F) -> crate::Result<T>
    where
        F: FnOnce(&mut PostgresConnection) -> crate::Result<T>,
    {
        let mut client = self.pool.get()?;
        let result = f(&mut client);
        result
    }
}

impl From<tokio_postgres::error::Error> for Error {
    fn from(e: tokio_postgres::error::Error) -> Error {
        use tokio_postgres::error::DbError;

        match e.code().map(|c| c.code()) {
            // Don't look at me, I'm hideous ;((
            Some("23505") => {
                let error = e.into_source().unwrap(); // boom
                let db_error = error.downcast_ref::<DbError>().unwrap(); // BOOM
                let detail = db_error.detail().unwrap(); // KA-BOOM

                let splitted: Vec<&str> = detail.split(")=(").collect();
                let splitted: Vec<&str> = splitted[0].split(" (").collect();
                let field_name = splitted[1].replace("\"", "");

                Error::UniqueConstraintViolation { field_name }
            }
            // Even lipstick will not save this...
            Some("23502") => {
                let error = e.into_source().unwrap(); // boom
                let db_error = error.downcast_ref::<DbError>().unwrap(); // BOOM
                let detail = db_error.detail().unwrap(); // KA-BOOM

                let splitted: Vec<&str> = detail.split(")=(").collect();
                let splitted: Vec<&str> = splitted[0].split(" (").collect();
                let field_name = splitted[1].replace("\"", "");

                Error::NullConstraintViolation { field_name }
            }
            _ => Error::QueryError(e.into()),
        }
    }
}

impl From<native_tls::Error> for Error {
    fn from(e: native_tls::Error) -> Error {
        Error::ConnectionError(e.into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[allow(unused)]
    fn get_config() -> Config {
        let mut config = Config::new();
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
                assert_eq!(res.into_iter().next().is_none(), true);

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
                assert_eq!(res.into_iter().next().is_none(), true);

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

                let res = connection.query_raw("SELECT * FROM \"user\"", &[]).unwrap();

                let mut result_count: u32 = 0;

                // Exactly one result expected.
                for row in &res {
                    assert_eq!(row.get_as_integer("id")?, 1);
                    assert_eq!(row.get_as_string("name")?, "Joe");
                    assert_eq!(row.get_as_integer("age")?, 27);
                    assert_eq!(row.get_as_real("salary")?, 20000.0);
                    result_count = result_count + 1;
                }

                assert_eq!(result_count, 1);

                Ok(())
            })
            .unwrap()
    }
}
