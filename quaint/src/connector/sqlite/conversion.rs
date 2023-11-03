use std::convert::TryFrom;

use crate::{
    ast::{Value, ValueType},
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
        self.decl_type().is_none()
    }
}

impl<'a> GetRow for SqliteRow<'a> {
    fn get_result_row(&self) -> crate::Result<Vec<Value<'static>>> {
        let statement = self.as_ref();
        let mut row = Vec::with_capacity(statement.columns().len());

        for (i, column) in statement.columns().iter().enumerate() {
            let pv = match self.get_ref_unwrap(i) {
                ValueRef::Null => match column {
                    // NOTE: A value without decl_type would be Int32(None)
                    c if c.is_int32() | c.is_null() => Value::null_int32(),
                    c if c.is_int64() => Value::null_int64(),
                    c if c.is_text() => Value::null_text(),
                    c if c.is_bytes() => Value::null_bytes(),
                    c if c.is_float() => Value::null_float(),
                    c if c.is_double() => Value::null_double(),
                    c if c.is_real() => Value::null_numeric(),
                    c if c.is_datetime() => Value::null_datetime(),
                    c if c.is_date() => Value::null_date(),
                    c if c.is_bool() => Value::null_boolean(),
                    c => match c.decl_type() {
                        Some(n) => {
                            let msg = format!("Value {n} not supported");
                            let kind = ErrorKind::conversion(msg);

                            return Err(Error::builder(kind).build());
                        }
                        // When we don't know what to do, the default value would be Int32(None)
                        None => Value::null_int32(),
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
                        c if c.is_date() => {
                            let dt = chrono::NaiveDateTime::from_timestamp_opt(i / 1000, 0).unwrap();
                            Value::date(dt.date())
                        }
                        c if c.is_datetime() => {
                            let dt = chrono::Utc.timestamp_millis_opt(i).unwrap();
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
                ValueRef::Real(f) if column.is_real() => {
                    use bigdecimal::{BigDecimal, FromPrimitive};

                    Value::numeric(BigDecimal::from_f64(f).unwrap())
                }
                ValueRef::Real(f) => Value::double(f),
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
        match self.as_ref() {
            Some(statement) => statement.column_names().into_iter().map(|c| c.into()).collect(),
            None => vec![],
        }
    }
}

impl<'a> ToSql for Value<'a> {
    fn to_sql(&self) -> Result<ToSqlOutput, RusqlError> {
        let value = match &self.typed {
            ValueType::Int32(integer) => integer.map(ToSqlOutput::from),
            ValueType::Int64(integer) => integer.map(ToSqlOutput::from),
            ValueType::Float(float) => float.map(|f| f as f64).map(ToSqlOutput::from),
            ValueType::Double(double) => double.map(ToSqlOutput::from),
            ValueType::Text(cow) => cow.as_ref().map(|cow| ToSqlOutput::from(cow.as_ref())),
            ValueType::Enum(cow, _) => cow.as_ref().map(|cow| ToSqlOutput::from(cow.as_ref())),
            ValueType::Boolean(boo) => boo.map(ToSqlOutput::from),
            ValueType::Char(c) => c.map(|c| ToSqlOutput::from(c as u8)),
            ValueType::Bytes(bytes) => bytes.as_ref().map(|bytes| ToSqlOutput::from(bytes.as_ref())),
            ValueType::Array(_) | ValueType::EnumArray(_, _) => {
                let msg = "Arrays are not supported in SQLite.";
                let kind = ErrorKind::conversion(msg);

                let mut builder = Error::builder(kind);
                builder.set_original_message(msg);

                return Err(RusqlError::ToSqlConversionFailure(Box::new(builder.build())));
            }
            ValueType::Numeric(d) => d
                .as_ref()
                .map(|d| ToSqlOutput::from(d.to_string().parse::<f64>().expect("BigDecimal is not a f64."))),
            ValueType::Json(value) => value.as_ref().map(|value| {
                let stringified = serde_json::to_string(value)
                    .map_err(|err| RusqlError::ToSqlConversionFailure(Box::new(err)))
                    .unwrap();

                ToSqlOutput::from(stringified)
            }),
            ValueType::Xml(cow) => cow.as_ref().map(|cow| ToSqlOutput::from(cow.as_ref())),
            ValueType::Uuid(value) => value.map(|value| ToSqlOutput::from(value.hyphenated().to_string())),
            ValueType::DateTime(value) => value.map(|value| ToSqlOutput::from(value.timestamp_millis())),
            ValueType::Date(date) => date
                .and_then(|date| date.and_hms_opt(0, 0, 0))
                .map(|dt| ToSqlOutput::from(dt.timestamp_millis())),
            ValueType::Time(time) => time
                .and_then(|time| chrono::NaiveDate::from_ymd_opt(1970, 1, 1).map(|d| (d, time)))
                .and_then(|(date, time)| {
                    use chrono::Timelike;
                    date.and_hms_opt(time.hour(), time.minute(), time.second())
                })
                .map(|dt| ToSqlOutput::from(dt.timestamp_millis())),
        };

        match value {
            Some(value) => Ok(value),
            None => Ok(ToSqlOutput::from(Null)),
        }
    }
}
