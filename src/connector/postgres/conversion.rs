use crate::{
    ast::{Id, ParameterizedValue},
    connector::queryable::{ToColumnNames, ToRow},
};
#[cfg(feature = "chrono-0_4")]
use chrono::{DateTime, NaiveDateTime, Utc};
use postgres::{
    types::{FromSql, ToSql, Type as PostgresType},
    Statement as PostgresStatement,
};
use rust_decimal::Decimal;
use tokio_postgres::Row as PostgresRow;

#[cfg(feature = "uuid-0_7")]
use uuid::Uuid;

pub fn conv_params<'a>(
    params: &'a [ParameterizedValue<'a>],
) -> Vec<&'a dyn tokio_postgres::types::ToSql> {
    params.iter().map(|x| x as &dyn ToSql).collect::<Vec<_>>()
}

#[cfg(feature = "uuid-0_7")]
fn accepts(ty: &PostgresType) -> bool {
    <Uuid as FromSql>::accepts(ty)
        || <&str as FromSql>::accepts(ty)
        || <i16 as FromSql>::accepts(ty)
        || <i32 as FromSql>::accepts(ty)
        || <i64 as FromSql>::accepts(ty)
}

#[cfg(not(feature = "uuid-0_7"))]
fn accepts(ty: &PostgresType) -> bool {
    <&str as FromSql>::accepts(ty)
        || <i16 as FromSql>::accepts(ty)
        || <i32 as FromSql>::accepts(ty)
        || <i64 as FromSql>::accepts(ty)
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
            #[cfg(feature = "uuid-0_7")]
            PostgresType::UUID => Id::UUID(Uuid::from_sql(ty, raw)?),
            _ => Id::String(String::from_sql(ty, raw)?),
        };

        Ok(res)
    }

    fn accepts(ty: &PostgresType) -> bool {
        accepts(ty)
    }
}

impl ToRow for PostgresRow {
    fn to_result_row<'b>(&'b self) -> crate::Result<Vec<ParameterizedValue<'static>>> {
        fn convert(row: &PostgresRow, i: usize) -> crate::Result<ParameterizedValue<'static>> {
            let result = match *row.columns()[i].type_() {
                PostgresType::BOOL => match row.try_get(i)? {
                    Some(val) => ParameterizedValue::Boolean(val),
                    None => ParameterizedValue::Null,
                },
                PostgresType::INT2 => match row.try_get(i)? {
                    Some(val) => {
                        let val: i16 = val;
                        ParameterizedValue::Integer(i64::from(val))
                    }
                    None => ParameterizedValue::Null,
                },
                PostgresType::INT4 => match row.try_get(i)? {
                    Some(val) => {
                        let val: i32 = val;
                        ParameterizedValue::Integer(i64::from(val))
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
                        ParameterizedValue::Real(f64::from(val))
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
                #[cfg(feature = "chrono-0_4")]
                PostgresType::TIMESTAMP => match row.try_get(i)? {
                    Some(val) => {
                        let ts: NaiveDateTime = val;
                        let dt = DateTime::<Utc>::from_utc(ts, Utc);
                        ParameterizedValue::DateTime(dt)
                    }
                    None => ParameterizedValue::Null,
                },
                #[cfg(feature = "uuid-0_7")]
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
                                .map(|x| ParameterizedValue::Integer(i64::from(x)))
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
                                .map(|x| ParameterizedValue::Integer(i64::from(x)))
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
                                .map(|x| ParameterizedValue::Real(f64::from(x)))
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
                            val.into_iter().map(ParameterizedValue::Boolean).collect(),
                        )
                    }
                    None => ParameterizedValue::Null,
                },
                #[cfg(all(feature = "array", feature = "chrono-0_4"))]
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
                PostgresType::OID => match row.try_get(i)? {
                    Some(val) => {
                        let val: u32 = val;
                        ParameterizedValue::Integer(i64::from(val))
                    }
                    None => ParameterizedValue::Null,
                },
                PostgresType::CHAR => match row.try_get(i)? {
                    Some(val) => {
                        let val: i8 = val;
                        ParameterizedValue::Char((val as u8) as char)
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

        let mut row = Vec::new();

        for i in 0..self.columns().len() {
            row.push(convert(self, i)?);
        }

        Ok(row)
    }
}

impl ToColumnNames for PostgresStatement {
    fn to_column_names(&self) -> Vec<String> {
        let mut names = Vec::new();

        for column in self.columns() {
            names.push(String::from(column.name()));
        }

        names
    }
}
