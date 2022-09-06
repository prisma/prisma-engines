use std::convert::TryFrom;

use crate::{
    ast::Value,
    connector::{
        queryable::{GetRow, ToColumnNames},
        TypeIdentifier,
    },
    error::{Error, ErrorKind},
};

use rusqlite::{
    types::{Null, ToSql, ToSqlOutput, ValueRef},
    Column, Error as RusqlError, Row as SqliteRow, Rows as SqliteRows,
};

#[cfg(feature = "chrono")]
use chrono::TimeZone;

impl TypeIdentifier for Column<'_> {
    fn is_real(&self) -> bool {
        match self.decl_type() {
            Some(n) if n.starts_with("DECIMAL") => true,
            Some(n) if n.starts_with("decimal") => true,
            _ => false,
        }
    }

    fn is_float(&self) -> bool {
        matches!(self.decl_type(), Some("FLOAT") | Some("float"))
    }

    fn is_double(&self) -> bool {
        matches!(
            self.decl_type(),
            Some("DOUBLE")
                | Some("double")
                | Some("DOUBLE PRECISION")
                | Some("double precision")
                | Some("numeric")
                | Some("NUMERIC")
                | Some("real")
                | Some("REAL")
        )
    }

    fn is_int32(&self) -> bool {
        matches!(
            self.decl_type(),
            Some("TINYINT")
                | Some("tinyint")
                | Some("SMALLINT")
                | Some("smallint")
                | Some("MEDIUMINT")
                | Some("mediumint")
                | Some("INT")
                | Some("int")
                | Some("INTEGER")
                | Some("integer")
                | Some("SERIAL")
                | Some("serial")
                | Some("INT2")
                | Some("int2")
        )
    }

    fn is_int64(&self) -> bool {
        matches!(
            self.decl_type(),
            Some("BIGINT")
                | Some("bigint")
                | Some("UNSIGNED BIG INT")
                | Some("unsigned big int")
                | Some("INT8")
                | Some("int8")
        )
    }

    fn is_datetime(&self) -> bool {
        matches!(
            self.decl_type(),
            Some("DATETIME") | Some("datetime") | Some("TIMESTAMP") | Some("timestamp")
        )
    }

    fn is_time(&self) -> bool {
        false
    }

    fn is_date(&self) -> bool {
        matches!(self.decl_type(), Some("DATE") | Some("date"))
    }

    fn is_text(&self) -> bool {
        match self.decl_type() {
            Some("TEXT") | Some("text") => true,
            Some("CLOB") | Some("clob") => true,
            Some(n) if n.starts_with("CHARACTER") => true,
            Some(n) if n.starts_with("character") => true,
            Some(n) if n.starts_with("VARCHAR") => true,
            Some(n) if n.starts_with("varchar") => true,
            Some(n) if n.starts_with("VARYING CHARACTER") => true,
            Some(n) if n.starts_with("varying character") => true,
            Some(n) if n.starts_with("NCHAR") => true,
            Some(n) if n.starts_with("nchar") => true,
            Some(n) if n.starts_with("NATIVE CHARACTER") => true,
            Some(n) if n.starts_with("native character") => true,
            Some(n) if n.starts_with("NVARCHAR") => true,
            Some(n) if n.starts_with("nvarchar") => true,
            _ => false,
        }
    }

    fn is_bytes(&self) -> bool {
        matches!(self.decl_type(), Some("BLOB") | Some("blob"))
    }

    fn is_bool(&self) -> bool {
        matches!(self.decl_type(), Some("BOOLEAN") | Some("boolean"))
    }

    fn is_json(&self) -> bool {
        false
    }
    fn is_enum(&self) -> bool {
        false
    }
    fn is_null(&self) -> bool {
        self.decl_type() == None
    }
}

impl<'a> GetRow for SqliteRow<'a> {
    fn get_result_row<'b>(&'b self) -> crate::Result<Vec<Value<'static>>> {
        let mut row = Vec::with_capacity(self.columns().len());

        for (i, column) in self.columns().iter().enumerate() {
            let pv = match self.get_ref_unwrap(i) {
                ValueRef::Null => match column {
                    // NOTE: A value without decl_type would be Int32(None)
                    c if c.is_int32() | c.is_null() => Value::Int32(None),
                    c if c.is_int64() => Value::Int64(None),
                    c if c.is_text() => Value::Text(None),
                    c if c.is_bytes() => Value::Bytes(None),
                    c if c.is_float() => Value::Float(None),
                    c if c.is_double() => Value::Double(None),
                    #[cfg(feature = "bigdecimal")]
                    c if c.is_real() => Value::Numeric(None),
                    #[cfg(feature = "chrono")]
                    c if c.is_datetime() => Value::DateTime(None),
                    #[cfg(feature = "chrono")]
                    c if c.is_date() => Value::Date(None),
                    c if c.is_bool() => Value::Boolean(None),
                    c => match c.decl_type() {
                        Some(n) => {
                            let msg = format!("Value {} not supported", n);
                            let kind = ErrorKind::conversion(msg);

                            return Err(Error::builder(kind).build());
                        }
                        // When we don't know what to do, the default value would be Int32(None)
                        None => Value::Int32(None),
                    },
                },
                ValueRef::Integer(i) => {
                    match column {
                        c if c.is_bool() => {
                            if i == 0 {
                                Value::boolean(false)
                            } else {
                                Value::boolean(true)
                            }
                        }
                        #[cfg(feature = "chrono")]
                        c if c.is_date() => {
                            let dt = chrono::NaiveDateTime::from_timestamp(i / 1000, 0);
                            Value::date(dt.date())
                        }
                        #[cfg(feature = "chrono")]
                        c if c.is_datetime() => {
                            let dt = chrono::Utc.timestamp_millis(i);
                            Value::datetime(dt)
                        }
                        c if c.is_int32() => {
                            if let Ok(converted) = i32::try_from(i) {
                                Value::int32(converted)
                            } else {
                                let msg = format!("Value {} does not fit in an INT column, try migrating the '{}' column type to BIGINT", i, c.name());
                                let kind = ErrorKind::conversion(msg);

                                return Err(Error::builder(kind).build());
                            }
                        }
                        // NOTE: When SQLite does not know what type the return is (for example at explicit values and RETURNING statements) we will 'assume' int64
                        _ => Value::int64(i),
                    }
                }
                #[cfg(feature = "bigdecimal")]
                ValueRef::Real(f) if column.is_real() => {
                    use bigdecimal::{BigDecimal, FromPrimitive};

                    Value::numeric(BigDecimal::from_f64(f).unwrap())
                }
                ValueRef::Real(f) => Value::double(f),
                #[cfg(feature = "chrono")]
                ValueRef::Text(bytes) if column.is_datetime() => {
                    let parse_res = std::str::from_utf8(bytes).map_err(|_| {
                        let builder = Error::builder(ErrorKind::ConversionError(
                            "Failed to read contents of SQLite datetime column as UTF-8".into(),
                        ));
                        builder.build()
                    });

                    parse_res.and_then(|s| {
                        chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S")
                            .map(|nd| chrono::DateTime::<chrono::Utc>::from_utc(nd, chrono::Utc))
                            .or_else(|_| {
                                chrono::DateTime::parse_from_rfc3339(s).map(|dt| dt.with_timezone(&chrono::Utc))
                            })
                            .or_else(|_| {
                                chrono::DateTime::parse_from_rfc2822(s).map(|dt| dt.with_timezone(&chrono::Utc))
                            })
                            .map(Value::datetime)
                            .map_err(|chrono_error| {
                                let builder =
                                    Error::builder(ErrorKind::ConversionError(chrono_error.to_string().into()));
                                builder.build()
                            })
                    })?
                }
                ValueRef::Text(bytes) => Value::text(String::from_utf8(bytes.to_vec())?),
                ValueRef::Blob(bytes) => Value::bytes(bytes.to_owned()),
            };

            row.push(pv);
        }

        Ok(row)
    }
}

impl<'a> ToColumnNames for SqliteRows<'a> {
    fn to_column_names(&self) -> Vec<String> {
        match self.column_names() {
            Some(columns) => columns.into_iter().map(|c| c.into()).collect(),
            None => vec![],
        }
    }
}

impl<'a> ToSql for Value<'a> {
    fn to_sql(&self) -> Result<ToSqlOutput, RusqlError> {
        let value = match self {
            Value::Int32(integer) => integer.map(ToSqlOutput::from),
            Value::Int64(integer) => integer.map(ToSqlOutput::from),
            Value::Float(float) => float.map(|f| f as f64).map(ToSqlOutput::from),
            Value::Double(double) => double.map(ToSqlOutput::from),
            Value::Text(cow) => cow.as_ref().map(|cow| ToSqlOutput::from(cow.as_ref())),
            Value::Enum(cow) => cow.as_ref().map(|cow| ToSqlOutput::from(cow.as_ref())),
            Value::Boolean(boo) => boo.map(ToSqlOutput::from),
            Value::Char(c) => c.map(|c| ToSqlOutput::from(c as u8)),
            Value::Bytes(bytes) => bytes.as_ref().map(|bytes| ToSqlOutput::from(bytes.as_ref())),
            Value::Array(_) => {
                let msg = "Arrays are not supported in SQLite.";
                let kind = ErrorKind::conversion(msg);

                let mut builder = Error::builder(kind);
                builder.set_original_message(msg);

                return Err(RusqlError::ToSqlConversionFailure(Box::new(builder.build())));
            }
            #[cfg(feature = "bigdecimal")]
            Value::Numeric(d) => d
                .as_ref()
                .map(|d| ToSqlOutput::from(d.to_string().parse::<f64>().expect("BigDecimal is not a f64."))),
            #[cfg(feature = "json")]
            Value::Json(value) => value.as_ref().map(|value| {
                let stringified = serde_json::to_string(value)
                    .map_err(|err| RusqlError::ToSqlConversionFailure(Box::new(err)))
                    .unwrap();

                ToSqlOutput::from(stringified)
            }),
            Value::Xml(cow) => cow.as_ref().map(|cow| ToSqlOutput::from(cow.as_ref())),
            #[cfg(feature = "uuid")]
            Value::Uuid(value) => value.map(|value| ToSqlOutput::from(value.hyphenated().to_string())),
            #[cfg(feature = "chrono")]
            Value::DateTime(value) => value.map(|value| ToSqlOutput::from(value.timestamp_millis())),
            #[cfg(feature = "chrono")]
            Value::Date(date) => date.map(|date| {
                let dt = date.and_hms(0, 0, 0);
                ToSqlOutput::from(dt.timestamp_millis())
            }),
            #[cfg(feature = "chrono")]
            Value::Time(time) => time.map(|time| {
                use chrono::{NaiveDate, Timelike};

                let dt = NaiveDate::from_ymd(1970, 1, 1).and_hms(time.hour(), time.minute(), time.second());

                ToSqlOutput::from(dt.timestamp_millis())
            }),
        };

        match value {
            Some(value) => Ok(value),
            None => Ok(ToSqlOutput::from(Null)),
        }
    }
}
