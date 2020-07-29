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
use rust_decimal::prelude::ToPrimitive;

impl TypeIdentifier for Column<'_> {
    fn is_real(&self) -> bool {
        match self.decl_type() {
            Some(n) if n.starts_with("DECIMAL") => true,
            Some(n) if n.starts_with("decimal") => true,
            Some("NUMERIC") | Some("REAL") | Some("DOUBLE") | Some("DOUBLE PRECISION") | Some("FLOAT") => true,
            Some("numeric") | Some("real") | Some("double") | Some("double precision") | Some("float") => true,
            _ => false,
        }
    }

    fn is_integer(&self) -> bool {
        matches!(
            self.decl_type(),
            Some("INT")
                | Some("int")
                | Some("INTEGER")
                | Some("integer")
                | Some("SERIAL")
                | Some("serial")
                | Some("TINYINT")
                | Some("tinyint")
                | Some("SMALLINT")
                | Some("smallint")
                | Some("MEDIUMINT")
                | Some("mediumint")
                | Some("BIGINT")
                | Some("bigint")
                | Some("UNSIGNED BIG INT")
                | Some("unsigned big int")
                | Some("INT2")
                | Some("int2")
                | Some("INT8")
                | Some("int8")
        )
    }

    fn is_datetime(&self) -> bool {
        matches!(self.decl_type(), Some("DATETIME") | Some("datetime"))
    }

    fn is_time(&self) -> bool {
        false
    }

    fn is_date(&self) -> bool {
        matches!(self.decl_type(), Some("DATE") | Some("date"))
    }

    fn is_text(&self) -> bool {
        match self.decl_type() {
            Some("TEXT") | Some("CLOB") => true,
            Some("text") | Some("clob") => true,
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
            let pv = match self.get_raw(i) {
                ValueRef::Null => match column {
                    c if c.is_integer() | c.is_null() => Value::Integer(None),
                    c if c.is_text() => Value::Text(None),
                    c if c.is_bytes() => Value::Bytes(None),
                    c if c.is_real() => Value::Real(None),
                    c if c.is_datetime() => Value::DateTime(None),
                    c if c.is_date() => Value::Date(None),
                    c if c.is_bool() => Value::Boolean(None),
                    c => match c.decl_type() {
                        Some(n) => {
                            let msg = format!("Value {} not supported", n);
                            let kind = ErrorKind::conversion(msg);

                            Err(Error::builder(kind).build())?
                        }
                        None => Value::Integer(None),
                    },
                },
                ValueRef::Integer(i) => match column {
                    c if c.is_bool() => {
                        if i == 0 {
                            Value::boolean(false)
                        } else {
                            Value::boolean(true)
                        }
                    }
                    #[cfg(feature = "chrono-0_4")]
                    c if c.is_date() => {
                        let dt = chrono::NaiveDateTime::from_timestamp(i / 1000, 0);
                        Value::date(dt.date())
                    }
                    #[cfg(feature = "chrono-0_4")]
                    c if c.is_datetime() => {
                        let sec = i / 1000;
                        let ns = i % 1000 * 1_000_000;
                        let dt = chrono::NaiveDateTime::from_timestamp(sec, ns as u32);
                        Value::datetime(chrono::DateTime::from_utc(dt, chrono::Utc))
                    }
                    _ => Value::integer(i),
                },
                ValueRef::Real(f) => Value::from(f),
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
            Value::Integer(integer) => integer.map(|i| ToSqlOutput::from(i)),
            Value::Real(d) => d.map(|d| ToSqlOutput::from(d.to_f64().expect("Decimal is not a f64."))),
            Value::Text(cow) => cow.as_ref().map(|cow| ToSqlOutput::from(cow.as_ref())),
            Value::Enum(cow) => cow.as_ref().map(|cow| ToSqlOutput::from(cow.as_ref())),
            Value::Boolean(boo) => boo.map(|boo| ToSqlOutput::from(boo)),
            Value::Char(c) => c.map(|c| ToSqlOutput::from(c as u8)),
            Value::Bytes(bytes) => bytes.as_ref().map(|bytes| ToSqlOutput::from(bytes.as_ref())),
            #[cfg(feature = "array")]
            Value::Array(_) => {
                let msg = "Arrays are not supported in SQLite.";
                let kind = ErrorKind::conversion(msg);

                let mut builder = Error::builder(kind);
                builder.set_original_message(msg);

                Err(RusqlError::ToSqlConversionFailure(Box::new(builder.build())))?
            }
            #[cfg(feature = "json-1")]
            Value::Json(value) => value.as_ref().map(|value| {
                let stringified = serde_json::to_string(value)
                    .map_err(|err| RusqlError::ToSqlConversionFailure(Box::new(err)))
                    .unwrap();

                ToSqlOutput::from(stringified)
            }),
            #[cfg(feature = "uuid-0_8")]
            Value::Uuid(value) => value.map(|value| ToSqlOutput::from(value.to_hyphenated().to_string())),
            #[cfg(feature = "chrono-0_4")]
            Value::DateTime(value) => value.map(|value| ToSqlOutput::from(value.timestamp_millis())),
            #[cfg(feature = "chrono-0_4")]
            Value::Date(date) => date.map(|date| {
                let dt = date.and_hms(0, 0, 0);
                ToSqlOutput::from(dt.timestamp_millis())
            }),
            #[cfg(feature = "chrono-0_4")]
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
