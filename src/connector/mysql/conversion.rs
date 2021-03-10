use crate::{
    ast::Value,
    connector::{queryable::TakeRow, TypeIdentifier},
    error::{Error, ErrorKind},
};
#[cfg(feature = "chrono")]
use chrono::{DateTime, Datelike, NaiveDate, NaiveDateTime, NaiveTime, Timelike, Utc};
use mysql_async::{
    self as my,
    consts::{ColumnFlags, ColumnType},
};
use std::convert::TryFrom;

#[tracing::instrument(skip(params))]
pub fn conv_params<'a>(params: &[Value<'a>]) -> crate::Result<my::Params> {
    if params.is_empty() {
        // If we don't use explicit 'Empty',
        // mysql crashes with 'internal error: entered unreachable code'
        Ok(my::Params::Empty)
    } else {
        let mut values = Vec::with_capacity(params.len());

        for pv in params {
            let res = match pv {
                Value::Integer(i) => i.map(my::Value::Int),
                Value::Float(f) => f.map(my::Value::Float),
                Value::Double(f) => f.map(my::Value::Double),
                Value::Text(s) => s.clone().map(|s| my::Value::Bytes((&*s).as_bytes().to_vec())),
                Value::Bytes(bytes) => bytes.clone().map(|bytes| my::Value::Bytes(bytes.into_owned())),
                Value::Enum(s) => s.clone().map(|s| my::Value::Bytes((&*s).as_bytes().to_vec())),
                Value::Boolean(b) => b.map(|b| my::Value::Int(b as i64)),
                Value::Char(c) => c.map(|c| my::Value::Bytes(vec![c as u8])),
                Value::Xml(s) => match s {
                    Some(ref s) => Some(my::Value::Bytes((s).as_bytes().to_vec())),
                    None => None,
                },
                Value::Array(_) => {
                    let msg = "Arrays are not supported in MySQL.";
                    let kind = ErrorKind::conversion(msg);

                    let mut builder = Error::builder(kind);
                    builder.set_original_message(msg);

                    return Err(builder.build());
                }
                #[cfg(feature = "bigdecimal")]
                Value::Numeric(f) => match f {
                    Some(f) => Some(my::Value::Bytes(f.to_string().as_bytes().to_vec())),
                    None => None,
                },
                #[cfg(feature = "json")]
                Value::Json(s) => match s {
                    Some(ref s) => {
                        let json = serde_json::to_string(s)?;
                        let bytes = json.into_bytes();

                        Some(my::Value::Bytes(bytes))
                    }
                    None => None,
                },
                #[cfg(feature = "uuid")]
                Value::Uuid(u) => u.map(|u| my::Value::Bytes(u.to_hyphenated().to_string().into_bytes())),
                #[cfg(feature = "chrono")]
                Value::Date(d) => {
                    d.map(|d| my::Value::Date(d.year() as u16, d.month() as u8, d.day() as u8, 0, 0, 0, 0))
                }
                #[cfg(feature = "chrono")]
                Value::Time(t) => {
                    t.map(|t| my::Value::Time(false, 0, t.hour() as u8, t.minute() as u8, t.second() as u8, 0))
                }
                #[cfg(feature = "chrono")]
                Value::DateTime(dt) => dt.map(|dt| {
                    my::Value::Date(
                        dt.year() as u16,
                        dt.month() as u8,
                        dt.day() as u8,
                        dt.hour() as u8,
                        dt.minute() as u8,
                        dt.second() as u8,
                        dt.timestamp_subsec_micros(),
                    )
                }),
            };

            match res {
                Some(val) => values.push(val),
                None => values.push(my::Value::NULL),
            }
        }

        Ok(my::Params::Positional(values))
    }
}

impl TypeIdentifier for my::Column {
    fn is_real(&self) -> bool {
        use ColumnType::*;

        matches!(self.column_type(), MYSQL_TYPE_DECIMAL | MYSQL_TYPE_NEWDECIMAL)
    }

    fn is_float(&self) -> bool {
        use ColumnType::*;

        matches!(self.column_type(), MYSQL_TYPE_FLOAT)
    }

    fn is_double(&self) -> bool {
        use ColumnType::*;

        matches!(self.column_type(), MYSQL_TYPE_DOUBLE)
    }

    fn is_integer(&self) -> bool {
        use ColumnType::*;

        matches!(
            self.column_type(),
            MYSQL_TYPE_TINY
                | MYSQL_TYPE_SHORT
                | MYSQL_TYPE_LONG
                | MYSQL_TYPE_LONGLONG
                | MYSQL_TYPE_YEAR
                | MYSQL_TYPE_INT24
        )
    }

    fn is_datetime(&self) -> bool {
        use ColumnType::*;

        matches!(
            self.column_type(),
            MYSQL_TYPE_TIMESTAMP | MYSQL_TYPE_DATETIME | MYSQL_TYPE_TIMESTAMP2 | MYSQL_TYPE_DATETIME2
        )
    }

    fn is_time(&self) -> bool {
        use ColumnType::*;

        matches!(self.column_type(), MYSQL_TYPE_TIME | MYSQL_TYPE_TIME2)
    }

    fn is_date(&self) -> bool {
        use ColumnType::*;

        matches!(self.column_type(), MYSQL_TYPE_DATE | MYSQL_TYPE_NEWDATE)
    }

    fn is_text(&self) -> bool {
        use ColumnType::*;

        let is_defined_text = matches!(
            self.column_type(),
            MYSQL_TYPE_VARCHAR | MYSQL_TYPE_VAR_STRING | MYSQL_TYPE_STRING
        );

        let is_bytes_but_text = matches!(
            self.column_type(),
            MYSQL_TYPE_TINY_BLOB | MYSQL_TYPE_MEDIUM_BLOB | MYSQL_TYPE_LONG_BLOB | MYSQL_TYPE_BLOB
        ) && self.character_set() != 63;

        is_defined_text || is_bytes_but_text
    }

    fn is_bytes(&self) -> bool {
        use ColumnType::*;

        let is_a_blob = matches!(
            self.column_type(),
            MYSQL_TYPE_TINY_BLOB | MYSQL_TYPE_MEDIUM_BLOB | MYSQL_TYPE_LONG_BLOB | MYSQL_TYPE_BLOB
        ) && self.character_set() == 63;

        let is_bits = self.column_type() == MYSQL_TYPE_BIT && self.column_length() > 1;

        is_a_blob || is_bits
    }

    fn is_bool(&self) -> bool {
        self.column_type() == ColumnType::MYSQL_TYPE_BIT && self.column_length() == 1
    }

    fn is_json(&self) -> bool {
        self.column_type() == ColumnType::MYSQL_TYPE_JSON
    }

    fn is_enum(&self) -> bool {
        self.flags() == ColumnFlags::ENUM_FLAG || self.column_type() == ColumnType::MYSQL_TYPE_ENUM
    }

    fn is_null(&self) -> bool {
        self.column_type() == ColumnType::MYSQL_TYPE_NULL
    }
}

impl TakeRow for my::Row {
    fn take_result_row(&mut self) -> crate::Result<Vec<Value<'static>>> {
        fn convert(row: &mut my::Row, i: usize) -> crate::Result<Value<'static>> {
            let value = row.take(i).ok_or_else(|| {
                let msg = "Index out of bounds";
                let kind = ErrorKind::conversion(msg);

                Error::builder(kind).build()
            })?;

            let column = row.columns_ref().get(i).ok_or_else(|| {
                let msg = "Index out of bounds";
                let kind = ErrorKind::conversion(msg);

                Error::builder(kind).build()
            })?;

            let res = match value {
                // JSON is returned as bytes.
                #[cfg(feature = "json")]
                my::Value::Bytes(b) if column.is_json() => {
                    serde_json::from_slice(&b).map(Value::json).map_err(|_| {
                        let msg = "Unable to convert bytes to JSON";
                        let kind = ErrorKind::conversion(msg);

                        Error::builder(kind).build()
                    })?
                }
                my::Value::Bytes(b) if column.is_enum() => {
                    let s = String::from_utf8(b)?;
                    Value::enum_variant(s)
                }
                // NEWDECIMAL returned as bytes. See https://mariadb.com/kb/en/resultset-row/#decimal-binary-encoding
                #[cfg(feature = "bigdecimal")]
                my::Value::Bytes(b) if column.is_real() => {
                    let s = String::from_utf8(b).map_err(|_| {
                        let msg = "Could not convert NEWDECIMAL from bytes to String.";
                        let kind = ErrorKind::conversion(msg);

                        Error::builder(kind).build()
                    })?;

                    let dec = s.parse().map_err(|_| {
                        let msg = "Could not convert NEWDECIMAL string to a BigDecimal.";
                        let kind = ErrorKind::conversion(msg);

                        Error::builder(kind).build()
                    })?;

                    Value::numeric(dec)
                }
                my::Value::Bytes(b) if column.is_bool() => match b.as_slice() {
                    [0] => Value::boolean(false),
                    _ => Value::boolean(true),
                },
                // https://dev.mysql.com/doc/internals/en/character-set.html
                my::Value::Bytes(b) if column.character_set() == 63 => Value::bytes(b),
                my::Value::Bytes(s) => Value::text(String::from_utf8(s)?),
                my::Value::Int(i) => Value::integer(i),
                my::Value::UInt(i) => Value::integer(i64::try_from(i).map_err(|_| {
                    let msg = "Unsigned integers larger than 9_223_372_036_854_775_807 are currently not handled.";
                    let kind = ErrorKind::value_out_of_range(msg);

                    Error::builder(kind).build()
                })?),
                my::Value::Float(f) => Value::from(f),
                my::Value::Double(f) => Value::from(f),
                #[cfg(feature = "chrono")]
                my::Value::Date(year, month, day, hour, min, sec, micro) => {
                    if day == 0 || month == 0 {
                        let msg = format!(
                            "The column `{}` contained an invalid datetime value with either day or month set to zero.",
                            column.name_str()
                        );
                        let kind = ErrorKind::value_out_of_range(msg);
                        return Err(Error::builder(kind).build());
                    }

                    let time = NaiveTime::from_hms_micro(hour.into(), min.into(), sec.into(), micro);

                    let date = NaiveDate::from_ymd(year.into(), month.into(), day.into());
                    let dt = NaiveDateTime::new(date, time);

                    Value::datetime(DateTime::<Utc>::from_utc(dt, Utc))
                }
                #[cfg(feature = "chrono")]
                my::Value::Time(is_neg, days, hours, minutes, seconds, micros) => {
                    if is_neg {
                        let kind = ErrorKind::conversion("Failed to convert a negative time");
                        return Err(Error::builder(kind).build());
                    }

                    if days != 0 {
                        let kind = ErrorKind::conversion("Failed to read a MySQL `time` as duration");
                        return Err(Error::builder(kind).build());
                    }

                    let time = NaiveTime::from_hms_micro(hours.into(), minutes.into(), seconds.into(), micros);
                    Value::time(time)
                }
                my::Value::NULL => match column {
                    t if t.is_bool() => Value::Boolean(None),
                    t if t.is_enum() => Value::Enum(None),
                    t if t.is_null() => Value::Integer(None),
                    t if t.is_integer() => Value::Integer(None),
                    t if t.is_float() => Value::Float(None),
                    t if t.is_double() => Value::Double(None),
                    t if t.is_text() => Value::Text(None),
                    t if t.is_bytes() => Value::Bytes(None),
                    #[cfg(feature = "bigdecimal")]
                    t if t.is_real() => Value::Numeric(None),
                    #[cfg(feature = "chrono")]
                    t if t.is_datetime() => Value::DateTime(None),
                    #[cfg(feature = "chrono")]
                    t if t.is_time() => Value::Time(None),
                    #[cfg(feature = "chrono")]
                    t if t.is_date() => Value::Date(None),
                    #[cfg(feature = "json")]
                    t if t.is_json() => Value::Json(None),
                    typ => {
                        let msg = format!(
                            "Value of type {:?} is not supported with the current configuration",
                            typ
                        );

                        let kind = ErrorKind::conversion(msg);
                        return Err(Error::builder(kind).build());
                    }
                },
                #[cfg(not(feature = "chrono"))]
                typ => {
                    let msg = format!(
                        "Value of type {:?} is not supported with the current configuration",
                        typ
                    );

                    let kind = ErrorKind::conversion(msg);
                    Err(Error::builder(kind).build())?
                }
            };

            Ok(res)
        }

        let mut row = Vec::with_capacity(self.len());

        for i in 0..self.len() {
            row.push(convert(self, i)?);
        }

        Ok(row)
    }
}
