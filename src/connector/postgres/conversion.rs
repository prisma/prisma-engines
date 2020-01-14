use crate::{
    ast::ParameterizedValue,
    connector::queryable::{GetRow, ToColumnNames},
};
use bytes::BytesMut;
#[cfg(feature = "chrono-0_4")]
use chrono::{DateTime, NaiveDateTime, Utc};
use rust_decimal::{prelude::FromPrimitive, Decimal};
use std::{error::Error, str::FromStr};
use tokio_postgres::{
    types::{self, IsNull, ToSql, Type as PostgresType},
    Row as PostgresRow, Statement as PostgresStatement,
};

#[cfg(feature = "uuid-0_8")]
use uuid::Uuid;

pub fn conv_params<'a>(params: &'a [ParameterizedValue<'a>]) -> Vec<&'a (dyn types::ToSql + Sync)> {
    params.iter().map(|x| x as &(dyn ToSql + Sync)).collect::<Vec<_>>()
}

impl GetRow for PostgresRow {
    fn get_result_row<'b>(&'b self) -> crate::Result<Vec<ParameterizedValue<'static>>> {
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
                        ParameterizedValue::Real(val)
                    }
                    None => ParameterizedValue::Null,
                },
                PostgresType::FLOAT4 => match row.try_get(i)? {
                    Some(val) => {
                        let val: Decimal = Decimal::from_f32(val).expect("f32 is not a Decimal");
                        ParameterizedValue::Real(val)
                    }
                    None => ParameterizedValue::Null,
                },
                PostgresType::FLOAT8 => match row.try_get(i)? {
                    Some(val) => {
                        let val: Decimal = Decimal::from_f64(val).expect("f64 is not a Decimal");
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
                #[cfg(feature = "uuid-0_8")]
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
                            val.into_iter().map(|x| ParameterizedValue::Integer(x as i64)).collect(),
                        )
                    }
                    None => ParameterizedValue::Null,
                },
                #[cfg(feature = "array")]
                PostgresType::FLOAT4_ARRAY => match row.try_get(i)? {
                    Some(val) => {
                        let val: Vec<f32> = val;
                        ParameterizedValue::Array(val.into_iter().map(ParameterizedValue::from).collect())
                    }
                    None => ParameterizedValue::Null,
                },
                #[cfg(feature = "array")]
                PostgresType::FLOAT8_ARRAY => match row.try_get(i)? {
                    Some(val) => {
                        let val: Vec<f64> = val;
                        ParameterizedValue::Array(val.into_iter().map(ParameterizedValue::from).collect())
                    }
                    None => ParameterizedValue::Null,
                },
                #[cfg(feature = "array")]
                PostgresType::BOOL_ARRAY => match row.try_get(i)? {
                    Some(val) => {
                        let val: Vec<bool> = val;
                        ParameterizedValue::Array(val.into_iter().map(ParameterizedValue::Boolean).collect())
                    }
                    None => ParameterizedValue::Null,
                },
                #[cfg(all(feature = "array", feature = "chrono-0_4"))]
                PostgresType::TIMESTAMP_ARRAY => match row.try_get(i)? {
                    Some(val) => {
                        let val: Vec<NaiveDateTime> = val;
                        ParameterizedValue::Array(
                            val.into_iter()
                                .map(|x| ParameterizedValue::DateTime(DateTime::<Utc>::from_utc(x, Utc)))
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
                PostgresType::TEXT_ARRAY | PostgresType::NAME_ARRAY | PostgresType::VARCHAR_ARRAY => {
                    match row.try_get(i)? {
                        Some(val) => {
                            let val: Vec<&str> = val;
                            ParameterizedValue::Array(
                                val.into_iter()
                                    .map(|x| ParameterizedValue::Text(String::from(x).into()))
                                    .collect(),
                            )
                        }
                        None => ParameterizedValue::Null,
                    }
                }
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

impl<'a> ToSql for ParameterizedValue<'a> {
    fn to_sql(&self, ty: &PostgresType, out: &mut BytesMut) -> Result<IsNull, Box<dyn Error + 'static + Send + Sync>> {
        match self {
            ParameterizedValue::Null => Ok(IsNull::Yes),
            ParameterizedValue::Integer(integer) => match *ty {
                PostgresType::INT2 => (*integer as i16).to_sql(ty, out),
                PostgresType::INT4 => (*integer as i32).to_sql(ty, out),
                PostgresType::TEXT => format!("{}", integer).to_sql(ty, out),
                _ => (*integer as i64).to_sql(ty, out),
            },
            ParameterizedValue::Real(float) => match *ty {
                PostgresType::NUMERIC => {
                    let s = float.to_string();
                    Decimal::from_str(&s).unwrap().to_sql(ty, out)
                }
                _ => float.to_sql(ty, out),
            },
            ParameterizedValue::Text(string) => string.to_sql(ty, out),
            ParameterizedValue::Boolean(boo) => boo.to_sql(ty, out),
            ParameterizedValue::Char(c) => (*c as i8).to_sql(ty, out),
            #[cfg(feature = "array")]
            ParameterizedValue::Array(vec) => vec.to_sql(ty, out),
            #[cfg(feature = "json-1")]
            ParameterizedValue::Json(value) => value.to_sql(ty, out),
            #[cfg(feature = "uuid-0_8")]
            ParameterizedValue::Uuid(value) => value.to_sql(ty, out),
            #[cfg(feature = "chrono-0_4")]
            ParameterizedValue::DateTime(value) => value.naive_utc().to_sql(ty, out),
        }
    }

    fn accepts(_: &PostgresType) -> bool {
        true // Please check later should we make this to be more restricted
    }

    fn to_sql_checked(
        &self,
        ty: &PostgresType,
        out: &mut BytesMut,
    ) -> Result<IsNull, Box<dyn Error + 'static + Send + Sync>> {
        match self {
            ParameterizedValue::Null => Ok(IsNull::Yes),
            ParameterizedValue::Integer(integer) => match *ty {
                PostgresType::INT2 => (*integer as i16).to_sql_checked(ty, out),
                PostgresType::INT4 => (*integer as i32).to_sql_checked(ty, out),
                PostgresType::TEXT => format!("{}", integer).to_sql(ty, out),
                _ => integer.to_sql_checked(ty, out),
            },
            ParameterizedValue::Real(float) => match *ty {
                PostgresType::NUMERIC => {
                    let s = float.to_string();
                    Decimal::from_str(&s).unwrap().to_sql(ty, out)
                }
                _ => float.to_sql(ty, out),
            },
            ParameterizedValue::Text(string) => string.to_sql_checked(ty, out),
            ParameterizedValue::Boolean(boo) => boo.to_sql_checked(ty, out),
            ParameterizedValue::Char(c) => (*c as i8).to_sql_checked(ty, out),
            #[cfg(feature = "array")]
            ParameterizedValue::Array(vec) => vec.to_sql_checked(ty, out),
            #[cfg(feature = "json-1")]
            ParameterizedValue::Json(value) => value.to_sql_checked(ty, out),
            #[cfg(feature = "uuid-0_8")]
            ParameterizedValue::Uuid(value) => value.to_sql_checked(ty, out),
            #[cfg(feature = "chrono-0_4")]
            ParameterizedValue::DateTime(value) => value.naive_utc().to_sql_checked(ty, out),
        }
    }
}
