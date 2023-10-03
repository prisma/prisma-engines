use std::borrow::Cow;
use std::str::FromStr;

use crate::async_js_function::AsyncJsFunction;
use crate::conversion::JSArg;
use crate::transaction::JsTransaction;
use napi::bindgen_prelude::{FromNapiValue, ToNapiValue};
use napi::threadsafe_function::{ErrorStrategy, ThreadsafeFunction};
use napi::{JsObject, JsString};
use napi_derive::napi;
use quaint::connector::ResultSet as QuaintResultSet;
use quaint::{
    error::{Error as QuaintError, ErrorKind},
    Value as QuaintValue, ValueInner as QuaintValueType,
};

// TODO(jkomyno): import these 3rd-party crates from the `quaint-core` crate.
use bigdecimal::{BigDecimal, FromPrimitive};
use chrono::{DateTime, Utc};
use chrono::{NaiveDate, NaiveTime};

/// Proxy is a struct wrapping a javascript object that exhibits basic primitives for
/// querying and executing SQL (i.e. a client connector). The Proxy uses NAPI ThreadSafeFunction to
/// invoke the code within the node runtime that implements the client connector.
pub(crate) struct CommonProxy {
    /// Execute a query given as SQL, interpolating the given parameters.
    query_raw: AsyncJsFunction<Query, JSResultSet>,

    /// Execute a query given as SQL, interpolating the given parameters and
    /// returning the number of affected rows.
    execute_raw: AsyncJsFunction<Query, u32>,

    /// Return the flavour for this driver.
    pub(crate) flavour: String,
}

/// This is a JS proxy for accessing the methods specific to top level
/// JS driver objects
pub(crate) struct DriverProxy {
    start_transaction: AsyncJsFunction<(), JsTransaction>,
}
/// This a JS proxy for accessing the methods, specific
/// to JS transaction objects
pub(crate) struct TransactionProxy {
    /// transaction options
    options: TransactionOptions,

    /// commit transaction
    commit: AsyncJsFunction<(), ()>,

    /// rollback transaction
    rollback: AsyncJsFunction<(), ()>,

    /// dispose transaction, cleanup logic executed at the end of the transaction lifecycle
    /// on drop.
    dispose: ThreadsafeFunction<(), ErrorStrategy::Fatal>,
}

/// This result set is more convenient to be manipulated from both Rust and NodeJS.
/// Quaint's version of ResultSet is:
///
/// pub struct ResultSet {
///     pub(crate) columns: Arc<Vec<String>>,
///     pub(crate) rows: Vec<Vec<Value<'static>>>,
///     pub(crate) last_insert_id: Option<u64>,
/// }
///
/// If we used this ResultSet would we would have worse ergonomics as quaint::Value is a structured
/// enum and cannot be used directly with the #[napi(Object)] macro. Thus requiring us to implement
/// the FromNapiValue and ToNapiValue traits for quaint::Value, and use a different custom type
/// representing the Value in javascript.
///
#[napi(object)]
#[derive(Debug)]
pub struct JSResultSet {
    pub column_types: Vec<ColumnType>,
    pub column_names: Vec<String>,
    // Note this might be encoded differently for performance reasons
    pub rows: Vec<Vec<serde_json::Value>>,
    pub last_insert_id: Option<String>,
}

impl JSResultSet {
    pub fn len(&self) -> usize {
        self.rows.len()
    }
}

#[napi]
#[derive(Debug)]
pub enum ColumnType {
    // [PLANETSCALE_TYPE] (MYSQL_TYPE) -> [TypeScript example]
    /// The following PlanetScale type IDs are mapped into Int32:
    /// - INT8 (TINYINT) -> e.g. `127`
    /// - INT16 (SMALLINT) -> e.g. `32767`
    /// - INT24 (MEDIUMINT) -> e.g. `8388607`
    /// - INT32 (INT) -> e.g. `2147483647`
    Int32 = 0,

    /// The following PlanetScale type IDs are mapped into Int64:
    /// - INT64 (BIGINT) -> e.g. `"9223372036854775807"` (String-encoded)
    Int64 = 1,

    /// The following PlanetScale type IDs are mapped into Float:
    /// - FLOAT32 (FLOAT) -> e.g. `3.402823466`
    Float = 2,

    /// The following PlanetScale type IDs are mapped into Double:
    /// - FLOAT64 (DOUBLE) -> e.g. `1.7976931348623157`
    Double = 3,

    /// The following PlanetScale type IDs are mapped into Numeric:
    /// - DECIMAL (DECIMAL) -> e.g. `"99999999.99"` (String-encoded)
    Numeric = 4,

    /// The following PlanetScale type IDs are mapped into Boolean:
    /// - BOOLEAN (BOOLEAN) -> e.g. `1`
    Boolean = 5,

    /// The following PlanetScale type IDs are mapped into Char:
    /// - CHAR (CHAR) -> e.g. `"c"` (String-encoded)
    Char = 6,

    /// The following PlanetScale type IDs are mapped into Text:
    /// - TEXT (TEXT) -> e.g. `"foo"` (String-encoded)
    /// - VARCHAR (VARCHAR) -> e.g. `"foo"` (String-encoded)
    Text = 7,

    /// The following PlanetScale type IDs are mapped into Date:
    /// - DATE (DATE) -> e.g. `"2023-01-01"` (String-encoded, yyyy-MM-dd)
    Date = 8,

    /// The following PlanetScale type IDs are mapped into Time:
    /// - TIME (TIME) -> e.g. `"23:59:59"` (String-encoded, HH:mm:ss)
    Time = 9,

    /// The following PlanetScale type IDs are mapped into DateTime:
    /// - DATETIME (DATETIME) -> e.g. `"2023-01-01 23:59:59"` (String-encoded, yyyy-MM-dd HH:mm:ss)
    /// - TIMESTAMP (TIMESTAMP) -> e.g. `"2023-01-01 23:59:59"` (String-encoded, yyyy-MM-dd HH:mm:ss)
    DateTime = 10,

    /// The following PlanetScale type IDs are mapped into Json:
    /// - JSON (JSON) -> e.g. `"{\"key\": \"value\"}"` (String-encoded)
    Json = 11,

    /// The following PlanetScale type IDs are mapped into Enum:
    /// - ENUM (ENUM) -> e.g. `"foo"` (String-encoded)
    Enum = 12,

    /// The following PlanetScale type IDs are mapped into Bytes:
    /// - BLOB (BLOB) -> e.g. `"\u0012"` (String-encoded)
    /// - VARBINARY (VARBINARY) -> e.g. `"\u0012"` (String-encoded)
    /// - BINARY (BINARY) -> e.g. `"\u0012"` (String-encoded)
    /// - GEOMETRY (GEOMETRY) -> e.g. `"\u0012"` (String-encoded)
    Bytes = 13,

    /// The following PlanetScale type IDs are mapped into Set:
    /// - SET (SET) -> e.g. `"foo,bar"` (String-encoded, comma-separated)
    /// This is currently unhandled, and will panic if encountered.
    Set = 14,

    /// UUID from postgres-flavored driver adapters is mapped to this type.
    Uuid = 15,

    /*
     * Scalar arrays
     */
    /// Int32 array (INT2_ARRAY and INT4_ARRAY in PostgreSQL)
    Int32Array = 64,

    /// Int64 array (INT8_ARRAY in PostgreSQL)
    Int64Array = 65,

    /// Float array (FLOAT4_ARRAY in PostgreSQL)
    FloatArray = 66,

    /// Double array (FLOAT8_ARRAY in PostgreSQL)
    DoubleArray = 67,

    /// Numeric array (NUMERIC_ARRAY, MONEY_ARRAY etc in PostgreSQL)
    NumericArray = 68,

    /// Boolean array (BOOL_ARRAY in PostgreSQL)
    BooleanArray = 69,

    /// Char array (CHAR_ARRAY in PostgreSQL)
    CharArray = 70,

    /// Text array (TEXT_ARRAY in PostgreSQL)
    TextArray = 71,

    /// Date array (DATE_ARRAY in PostgreSQL)
    DateArray = 72,

    /// Time array (TIME_ARRAY in PostgreSQL)
    TimeArray = 73,

    /// DateTime array (TIMESTAMP_ARRAY in PostgreSQL)
    DateTimeArray = 74,

    /// Json array (JSON_ARRAY in PostgreSQL)
    JsonArray = 75,

    /// Enum array
    EnumArray = 76,

    /// Bytes array (BYTEA_ARRAY in PostgreSQL)
    BytesArray = 77,

    /// Uuid array (UUID_ARRAY in PostgreSQL)
    UuidArray = 78,

    /*
     * Below there are custom types that don't have a 1:1 translation with a quaint::Value.
     * enum variant.
     */
    /// UnknownNumber is used when the type of the column is a number but of unknown particular type
    /// and precision.
    ///
    /// It's used by some driver adapters, like libsql to return aggregation values like AVG, or
    /// COUNT, and it can be mapped to either Int64, or Double
    UnknownNumber = 128,
}

#[napi(object)]
#[derive(Debug)]
pub struct Query {
    pub sql: String,
    pub args: Vec<JSArg>,
}

fn conversion_error(args: &std::fmt::Arguments) -> QuaintError {
    let msg = match args.as_str() {
        Some(s) => Cow::Borrowed(s),
        None => Cow::Owned(args.to_string()),
    };
    QuaintError::builder(ErrorKind::ConversionError(msg)).build()
}

macro_rules! conversion_error {
    ($($arg:tt)*) => {
        conversion_error(&format_args!($($arg)*))
    };
}

/// Handle data-type conversion from a JSON value to a Quaint value.
/// This is used for most data types, except those that require connector-specific handling, e.g., `ColumnType::Boolean`.
fn js_value_to_quaint(
    json_value: serde_json::Value,
    column_type: ColumnType,
    column_name: &str,
) -> quaint::Result<QuaintValue<'static>> {
    //  Note for the future: it may be worth revisiting how much bloat so many panics with different static
    // strings add to the compiled artefact, and in case we should come up with a restricted set of panic
    // messages, or even find a way of removing them altogether.
    match column_type {
        ColumnType::Int32 => match json_value {
            serde_json::Value::Number(n) => {
                // n.as_i32() is not implemented, so we need to downcast from i64 instead
                n.as_i64()
                    .ok_or(conversion_error!("number must be an integer"))
                    .and_then(|n| -> quaint::Result<i32> {
                        n.try_into()
                            .map_err(|e| conversion_error!("cannot convert {n} to i32: {e}"))
                    })
                    .map(QuaintValue::int32)
            }
            serde_json::Value::String(s) => s
                .parse::<i32>()
                .map(QuaintValue::int32)
                .map_err(|e| conversion_error!("string-encoded number must be an i32, got {s}: {e}")),
            serde_json::Value::Null => Ok(QuaintValueType::Int32(None).into()),
            mismatch => Err(conversion_error!(
                "expected an i32 number in column {column_name}, found {mismatch}"
            )),
        },
        ColumnType::Int64 => match json_value {
            serde_json::Value::Number(n) => n
                .as_i64()
                .map(QuaintValue::int64)
                .ok_or(conversion_error!("number must be an i64, got {n}")),
            serde_json::Value::String(s) => s
                .parse::<i64>()
                .map(QuaintValue::int64)
                .map_err(|e| conversion_error!("string-encoded number must be an i64, got {s}: {e}")),
            serde_json::Value::Null => Ok(QuaintValueType::Int64(None).into()),
            mismatch => Err(conversion_error!(
                "expected a string or number in column {column_name}, found {mismatch}"
            )),
        },
        ColumnType::Float => match json_value {
            // n.as_f32() is not implemented, so we need to downcast from f64 instead.
            // We assume that the JSON value is a valid f32 number, but we check for overflows anyway.
            serde_json::Value::Number(n) => n
                .as_f64()
                .ok_or(conversion_error!("number must be a float, got {n}"))
                .and_then(f64_to_f32)
                .map(QuaintValue::float),
            serde_json::Value::Null => Ok(QuaintValueType::Float(None).into()),
            mismatch => Err(conversion_error!(
                "expected an f32 number in column {column_name}, found {mismatch}"
            )),
        },
        ColumnType::Double => match json_value {
            serde_json::Value::Number(n) => n
                .as_f64()
                .map(QuaintValue::double)
                .ok_or(conversion_error!("number must be a f64, got {n}")),
            serde_json::Value::Null => Ok(QuaintValueType::Double(None).into()),
            mismatch => Err(conversion_error!(
                "expected an f64 number in column {column_name}, found {mismatch}"
            )),
        },
        ColumnType::Numeric => match json_value {
            serde_json::Value::String(s) => BigDecimal::from_str(&s)
                .map(QuaintValue::numeric)
                .map_err(|e| conversion_error!("invalid numeric value when parsing {s}: {e}")),
            serde_json::Value::Number(n) => n
                .as_f64()
                .and_then(BigDecimal::from_f64)
                .ok_or(conversion_error!("number must be an f64, got {n}"))
                .map(QuaintValue::numeric),
            serde_json::Value::Null => Ok(QuaintValueType::Numeric(None).into()),
            mismatch => Err(conversion_error!(
                "expected a string-encoded number in column {column_name}, found {mismatch}",
            )),
        },
        ColumnType::Boolean => match json_value {
            serde_json::Value::Bool(b) => Ok(QuaintValue::boolean(b)),
            serde_json::Value::Null => Ok(QuaintValueType::Boolean(None).into()),
            serde_json::Value::Number(n) => match n.as_i64() {
                Some(0) => Ok(QuaintValue::boolean(false)),
                Some(1) => Ok(QuaintValue::boolean(true)),
                _ => Err(conversion_error!(
                    "expected number-encoded boolean to be 0 or 1, got {n}"
                )),
            },
            serde_json::Value::String(s) => match s.as_str() {
                "false" | "FALSE" | "0" => Ok(QuaintValue::boolean(false)),
                "true" | "TRUE" | "1" => Ok(QuaintValue::boolean(true)),
                _ => Err(conversion_error!("expected string-encoded boolean, got {s}")),
            },
            mismatch => Err(conversion_error!(
                "expected a boolean in column {column_name}, found {mismatch}"
            )),
        },
        ColumnType::Char => match json_value {
            serde_json::Value::String(s) => Ok(QuaintValueType::Char(s.chars().next()).into()),
            serde_json::Value::Null => Ok(QuaintValueType::Char(None).into()),
            mismatch => Err(conversion_error!(
                "expected a string in column {column_name}, found {mismatch}"
            )),
        },
        ColumnType::Text => match json_value {
            serde_json::Value::String(s) => Ok(QuaintValue::text(s)),
            serde_json::Value::Null => Ok(QuaintValueType::Text(None).into()),
            mismatch => Err(conversion_error!(
                "expected a string in column {column_name}, found {mismatch}"
            )),
        },
        ColumnType::Date => match json_value {
            serde_json::Value::String(s) => NaiveDate::parse_from_str(&s, "%Y-%m-%d")
                .map(QuaintValue::date)
                .map_err(|_| conversion_error!("expected a date string, got {s}")),
            serde_json::Value::Null => Ok(QuaintValueType::Date(None).into()),
            mismatch => Err(conversion_error!(
                "expected a string in column {column_name}, found {mismatch}"
            )),
        },
        ColumnType::Time => match json_value {
            serde_json::Value::String(s) => NaiveTime::parse_from_str(&s, "%H:%M:%S")
                .map(QuaintValue::time)
                .map_err(|_| conversion_error!("expected a time string, got {s}")),
            serde_json::Value::Null => Ok(QuaintValueType::Time(None).into()),
            mismatch => Err(conversion_error!(
                "expected a string in column {column_name}, found {mismatch}"
            )),
        },
        ColumnType::DateTime => match json_value {
            serde_json::Value::String(s) => chrono::NaiveDateTime::parse_from_str(&s, "%Y-%m-%d %H:%M:%S%.f")
                .map(|dt| DateTime::from_utc(dt, Utc))
                .or_else(|_| DateTime::parse_from_rfc3339(&s).map(DateTime::<Utc>::from))
                .map(QuaintValue::datetime)
                .map_err(|_| conversion_error!("expected a datetime string, found {s}")),
            serde_json::Value::Null => Ok(QuaintValueType::DateTime(None).into()),
            mismatch => Err(conversion_error!(
                "expected a string in column {column_name}, found {mismatch}"
            )),
        },
        ColumnType::Json => {
            match json_value {
                // DbNull
                serde_json::Value::Null => Ok(QuaintValueType::Json(None).into()),
                // JsonNull
                serde_json::Value::String(s) if s == "$__prisma_null" => Ok(QuaintValue::json(serde_json::Value::Null)),
                json => Ok(QuaintValue::json(json)),
            }
        }
        ColumnType::Enum => match json_value {
            serde_json::Value::String(s) => Ok(QuaintValue::enum_variant(s)),
            serde_json::Value::Null => Ok(QuaintValueType::Enum(None, None).into()),
            mismatch => Err(conversion_error!(
                "expected a string in column {column_name}, found {mismatch}"
            )),
        },
        ColumnType::Bytes => match json_value {
            serde_json::Value::String(s) => Ok(QuaintValueType::Bytes(Some(s.into_bytes().into())).into()),
            serde_json::Value::Array(array) => array
                .iter()
                .map(|value| value.as_i64().and_then(|maybe_byte| maybe_byte.try_into().ok()))
                .collect::<Option<Cow<[u8]>>>()
                .map(QuaintValue::bytes)
                .ok_or(conversion_error!("elements of the array must be u8")),
            serde_json::Value::Null => Ok(QuaintValueType::Bytes(None).into()),
            mismatch => Err(conversion_error!(
                "expected a string or an array in column {column_name}, found {mismatch}",
            )),
        },
        ColumnType::Uuid => match json_value {
            serde_json::Value::String(s) => uuid::Uuid::parse_str(&s)
                .map(QuaintValue::uuid)
                .map_err(|_| conversion_error!("Expected a UUID string")),
            serde_json::Value::Null => Ok(QuaintValueType::Bytes(None).into()),
            mismatch => Err(conversion_error!(
                "Expected a UUID string in column {column_name}, found {mismatch}"
            )),
        },
        ColumnType::UnknownNumber => match json_value {
            serde_json::Value::Number(n) => n
                .as_i64()
                .map(QuaintValue::int64)
                .or(n.as_f64().map(QuaintValue::double))
                .ok_or(conversion_error!("number must be an i64 or f64, got {n}")),
            mismatch => Err(conversion_error!(
                "expected a either an i64 or a f64 in column {column_name}, found {mismatch}",
            )),
        },

        ColumnType::Int32Array => js_array_to_quaint(ColumnType::Int32, json_value, column_name),
        ColumnType::Int64Array => js_array_to_quaint(ColumnType::Int64, json_value, column_name),
        ColumnType::FloatArray => js_array_to_quaint(ColumnType::Float, json_value, column_name),
        ColumnType::DoubleArray => js_array_to_quaint(ColumnType::Double, json_value, column_name),
        ColumnType::NumericArray => js_array_to_quaint(ColumnType::Numeric, json_value, column_name),
        ColumnType::BooleanArray => js_array_to_quaint(ColumnType::Boolean, json_value, column_name),
        ColumnType::CharArray => js_array_to_quaint(ColumnType::Char, json_value, column_name),
        ColumnType::TextArray => js_array_to_quaint(ColumnType::Text, json_value, column_name),
        ColumnType::DateArray => js_array_to_quaint(ColumnType::Date, json_value, column_name),
        ColumnType::TimeArray => js_array_to_quaint(ColumnType::Time, json_value, column_name),
        ColumnType::DateTimeArray => js_array_to_quaint(ColumnType::DateTime, json_value, column_name),
        ColumnType::JsonArray => js_array_to_quaint(ColumnType::Json, json_value, column_name),
        ColumnType::EnumArray => js_array_to_quaint(ColumnType::Enum, json_value, column_name),
        ColumnType::BytesArray => js_array_to_quaint(ColumnType::Bytes, json_value, column_name),
        ColumnType::UuidArray => js_array_to_quaint(ColumnType::Uuid, json_value, column_name),

        unimplemented => {
            todo!("support column type {:?} in column {}", unimplemented, column_name)
        }
    }
}

fn js_array_to_quaint(
    base_type: ColumnType,
    json_value: serde_json::Value,
    column_name: &str,
) -> quaint::Result<QuaintValue<'static>> {
    match json_value {
        serde_json::Value::Array(array) => Ok(QuaintValue::Array(Some(
            array
                .into_iter()
                .enumerate()
                .map(|(index, elem)| js_value_to_quaint(elem, base_type, &format!("{column_name}[{index}]")))
                .collect::<quaint::Result<Vec<_>>>()?,
        ))),
        serde_json::Value::Null => Ok(QuaintValue::Array(None)),
        mismatch => Err(conversion_error!(
            "expected an array in column {column_name}, found {mismatch}",
        )),
    }
}

impl TryFrom<JSResultSet> for QuaintResultSet {
    type Error = quaint::error::Error;

    fn try_from(js_result_set: JSResultSet) -> Result<Self, Self::Error> {
        let JSResultSet {
            rows,
            column_names,
            column_types,
            last_insert_id,
        } = js_result_set;

        let mut quaint_rows = Vec::with_capacity(rows.len());

        for row in rows {
            let mut quaint_row = Vec::with_capacity(column_types.len());

            for (i, row) in row.into_iter().enumerate() {
                let column_type = column_types[i];
                let column_name = column_names[i].as_str();

                quaint_row.push(js_value_to_quaint(row, column_type, column_name)?);
            }

            quaint_rows.push(quaint_row);
        }

        let last_insert_id = last_insert_id.and_then(|id| id.parse::<u64>().ok());
        let mut quaint_result_set = QuaintResultSet::new(column_names, quaint_rows);

        // Not a fan of this (extracting the `Some` value from an `Option` and pass it to a method that creates a new `Some` value),
        // but that's Quaint's ResultSet API and that's how the MySQL connector does it.
        // Sqlite, on the other hand, uses a `last_insert_id.unwrap_or(0)` approach.
        if let Some(last_insert_id) = last_insert_id {
            quaint_result_set.set_last_insert_id(last_insert_id);
        }

        Ok(quaint_result_set)
    }
}

impl CommonProxy {
    pub fn new(object: &JsObject) -> napi::Result<Self> {
        let flavour: JsString = object.get_named_property("flavour")?;

        Ok(Self {
            query_raw: object.get_named_property("queryRaw")?,
            execute_raw: object.get_named_property("executeRaw")?,
            flavour: flavour.into_utf8()?.as_str()?.to_owned(),
        })
    }

    pub async fn query_raw(&self, params: Query) -> quaint::Result<JSResultSet> {
        self.query_raw.call(params).await
    }

    pub async fn execute_raw(&self, params: Query) -> quaint::Result<u32> {
        self.execute_raw.call(params).await
    }
}

impl DriverProxy {
    pub fn new(driver_adapter: &JsObject) -> napi::Result<Self> {
        Ok(Self {
            start_transaction: driver_adapter.get_named_property("startTransaction")?,
        })
    }

    pub async fn start_transaction(&self) -> quaint::Result<Box<JsTransaction>> {
        let tx = self.start_transaction.call(()).await?;
        Ok(Box::new(tx))
    }
}

#[derive(Debug)]
#[napi(object)]
pub struct TransactionOptions {
    /// Whether or not to run a phantom query (i.e., a query that only influences Prisma event logs, but not the database itself)
    /// before opening a transaction, committing, or rollbacking.
    pub use_phantom_query: bool,
}

impl TransactionProxy {
    pub fn new(js_transaction: &JsObject) -> napi::Result<Self> {
        let commit = js_transaction.get_named_property("commit")?;
        let rollback = js_transaction.get_named_property("rollback")?;
        let dispose = js_transaction.get_named_property("dispose")?;
        let options = js_transaction.get_named_property("options")?;

        Ok(Self {
            commit,
            rollback,
            dispose,
            options,
        })
    }

    pub fn options(&self) -> &TransactionOptions {
        &self.options
    }

    pub async fn commit(&self) -> quaint::Result<()> {
        self.commit.call(()).await
    }

    pub async fn rollback(&self) -> quaint::Result<()> {
        self.rollback.call(()).await
    }
}

impl Drop for TransactionProxy {
    fn drop(&mut self) {
        _ = self
            .dispose
            .call((), napi::threadsafe_function::ThreadsafeFunctionCallMode::NonBlocking);
    }
}

/// Coerce a `f64` to a `f32`, asserting that the conversion is lossless.
/// Note that, when overflow occurs during conversion, the result is `infinity`.
fn f64_to_f32(x: f64) -> quaint::Result<f32> {
    let y = x as f32;

    if x.is_finite() == y.is_finite() {
        Ok(y)
    } else {
        Err(conversion_error!("f32 overflow during conversion"))
    }
}
#[cfg(test)]
mod proxy_test {
    use num_bigint::BigInt;
    use serde_json::json;

    use super::*;

    #[track_caller]
    fn test_null(quaint_none: QuaintValue, column_type: ColumnType) {
        let json_value = serde_json::Value::Null;
        let quaint_value = js_value_to_quaint(json_value, column_type, "column_name").unwrap();
        assert_eq!(quaint_value, quaint_none);
    }

    #[test]
    fn js_value_int32_to_quaint() {
        let column_type = ColumnType::Int32;

        // null
        test_null(QuaintValueType::Int32(None).into(), column_type);

        // 0
        let n: i32 = 0;
        let json_value = serde_json::Value::Number(serde_json::Number::from(n));
        let quaint_value = js_value_to_quaint(json_value, column_type, "column_name").unwrap();
        assert_eq!(quaint_value.inner, QuaintValueKind::Int32(Some(n)));

        // max
        let n: i32 = i32::MAX;
        let json_value = serde_json::Value::Number(serde_json::Number::from(n));
        let quaint_value = js_value_to_quaint(json_value, column_type, "column_name").unwrap();
        assert_eq!(quaint_value.inner, QuaintValueKind::Int32(Some(n)));

        // min
        let n: i32 = i32::MIN;
        let json_value = serde_json::Value::Number(serde_json::Number::from(n));
        let quaint_value = js_value_to_quaint(json_value, column_type, "column_name").unwrap();
        assert_eq!(quaint_value.inner, QuaintValueKind::Int32(Some(n)));

        // string-encoded
        let n = i32::MAX;
        let json_value = serde_json::Value::String(n.to_string());
        let quaint_value = js_value_to_quaint(json_value, column_type, "column_name").unwrap();
        assert_eq!(quaint_value.inner, QuaintValueKind::Int32(Some(n)));
    }

    #[test]
    fn js_value_int64_to_quaint() {
        let column_type = ColumnType::Int64;

        // null
        test_null(QuaintValueType::Int64(None).into(), column_type);

        // 0
        let n: i64 = 0;
        let json_value = serde_json::Value::String(n.to_string());
        let quaint_value = js_value_to_quaint(json_value, column_type, "column_name").unwrap();
        assert_eq!(quaint_value.inner, QuaintValueKind::Int64(Some(n)));

        // max
        let n: i64 = i64::MAX;
        let json_value = serde_json::Value::String(n.to_string());
        let quaint_value = js_value_to_quaint(json_value, column_type, "column_name").unwrap();
        assert_eq!(quaint_value.inner, QuaintValueKind::Int64(Some(n)));

        // min
        let n: i64 = i64::MIN;
        let json_value = serde_json::Value::String(n.to_string());
        let quaint_value = js_value_to_quaint(json_value, column_type, "column_name").unwrap();
        assert_eq!(quaint_value.inner, QuaintValueKind::Int64(Some(n)));

        // number-encoded
        let n: i64 = (1 << 53) - 1; // max JS safe integer
        let json_value = serde_json::Value::Number(serde_json::Number::from(n));
        let quaint_value = js_value_to_quaint(json_value, column_type, "column_name").unwrap();
        assert_eq!(quaint_value.inner, QuaintValueKind::Int64(Some(n)));
    }

    #[test]
    fn js_value_float_to_quaint() {
        let column_type = ColumnType::Float;

        // null
        test_null(QuaintValueType::Float(None).into(), column_type);

        // 0
        let n: f32 = 0.0;
        let json_value = serde_json::Value::Number(serde_json::Number::from_f64(n.into()).unwrap());
        let quaint_value = js_value_to_quaint(json_value, column_type, "column_name").unwrap();
        assert_eq!(quaint_value.inner, QuaintValueKind::Float(Some(n)));

        // max
        let n: f32 = f32::MAX;
        let json_value = serde_json::Value::Number(serde_json::Number::from_f64(n.into()).unwrap());
        let quaint_value = js_value_to_quaint(json_value, column_type, "column_name").unwrap();
        assert_eq!(quaint_value.inner, QuaintValueKind::Float(Some(n)));

        // min
        let n: f32 = f32::MIN;
        let json_value = serde_json::Value::Number(serde_json::Number::from_f64(n.into()).unwrap());
        let quaint_value = js_value_to_quaint(json_value, column_type, "column_name").unwrap();
        assert_eq!(quaint_value.inner, QuaintValueKind::Float(Some(n)));
    }

    #[test]
    fn js_value_double_to_quaint() {
        let column_type = ColumnType::Double;

        // null
        test_null(QuaintValueType::Double(None).into(), column_type);

        // 0
        let n: f64 = 0.0;
        let json_value = serde_json::Value::Number(serde_json::Number::from_f64(n).unwrap());
        let quaint_value = js_value_to_quaint(json_value, column_type, "column_name").unwrap();
        assert_eq!(quaint_value.inner, QuaintValueKind::Double(Some(n)));

        // max
        let n: f64 = f64::MAX;
        let json_value = serde_json::Value::Number(serde_json::Number::from_f64(n).unwrap());
        let quaint_value = js_value_to_quaint(json_value, column_type, "column_name").unwrap();
        assert_eq!(quaint_value.inner, QuaintValueKind::Double(Some(n)));

        // min
        let n: f64 = f64::MIN;
        let json_value = serde_json::Value::Number(serde_json::Number::from_f64(n).unwrap());
        let quaint_value = js_value_to_quaint(json_value, column_type, "column_name").unwrap();
        assert_eq!(quaint_value.inner, QuaintValueKind::Double(Some(n)));
    }

    #[test]
    fn js_value_numeric_to_quaint() {
        let column_type = ColumnType::Numeric;

        // null
        test_null(QuaintValueType::Numeric(None).into(), column_type);

        let n_as_string = "1234.99";
        let decimal = BigDecimal::new(BigInt::parse_bytes(b"123499", 10).unwrap(), 2);

        let json_value = serde_json::Value::String(n_as_string.into());
        let quaint_value = js_value_to_quaint(json_value, column_type, "column_name").unwrap();
        assert_eq!(quaint_value.inner, QuaintValueKind::Numeric(Some(decimal)));

        let n_as_string = "1234.999999";
        let decimal = BigDecimal::new(BigInt::parse_bytes(b"1234999999", 10).unwrap(), 6);

        let json_value = serde_json::Value::String(n_as_string.into());
        let quaint_value = js_value_to_quaint(json_value, column_type, "column_name").unwrap();
        assert_eq!(quaint_value.inner, QuaintValueKind::Numeric(Some(decimal)));
    }

    #[test]
    fn js_value_boolean_to_quaint() {
        let column_type = ColumnType::Boolean;

        // null
        test_null(QuaintValueType::Boolean(None).into(), column_type);

        // true
        for truthy_value in [json!(true), json!(1), json!("true"), json!("TRUE"), json!("1")] {
            let quaint_value = js_value_to_quaint(truthy_value, column_type, "column_name").unwrap();
            assert_eq!(quaint_value.inner, QuaintValueKind::Boolean(Some(true)));
        }

        // false
        for falsy_value in [json!(false), json!(0), json!("false"), json!("FALSE"), json!("0")] {
            let quaint_value = js_value_to_quaint(falsy_value, column_type, "column_name").unwrap();
            assert_eq!(quaint_value.inner, QuaintValueKind::Boolean(Some(false)));
        }
    }

    #[test]
    fn js_value_char_to_quaint() {
        let column_type = ColumnType::Char;

        // null
        test_null(QuaintValueType::Char(None).into(), column_type);

        let c = 'c';
        let json_value = serde_json::Value::String(c.to_string());
        let quaint_value = js_value_to_quaint(json_value, column_type, "column_name").unwrap();
        assert_eq!(quaint_value.inner, QuaintValueKind::Char(Some(c)));
    }

    #[test]
    fn js_value_text_to_quaint() {
        let column_type = ColumnType::Text;

        // null
        test_null(QuaintValueType::Text(None).into(), column_type);

        let s = "some text";
        let json_value = serde_json::Value::String(s.to_string());
        let quaint_value = js_value_to_quaint(json_value, column_type, "column_name").unwrap();
        assert_eq!(quaint_value.inner, QuaintValueKind::Text(Some(s.into())));
    }

    #[test]
    fn js_value_date_to_quaint() {
        let column_type = ColumnType::Date;

        // null
        test_null(QuaintValueType::Date(None).into(), column_type);

        let s = "2023-01-01";
        let json_value = serde_json::Value::String(s.to_string());
        let quaint_value = js_value_to_quaint(json_value, column_type, "column_name").unwrap();

        let date = NaiveDate::from_ymd_opt(2023, 1, 1).unwrap();
        assert_eq!(quaint_value.inner, QuaintValueKind::Date(Some(date)));
    }

    #[test]
    fn js_value_time_to_quaint() {
        let column_type = ColumnType::Time;

        // null
        test_null(QuaintValueType::Time(None).into(), column_type);

        let s = "23:59:59";
        let json_value = serde_json::Value::String(s.to_string());
        let quaint_value = js_value_to_quaint(json_value, column_type, "column_name").unwrap();

        let time: NaiveTime = NaiveTime::from_hms_opt(23, 59, 59).unwrap();
        assert_eq!(quaint_value.inner, QuaintValueKind::Time(Some(time)));
    }

    #[test]
    fn js_value_datetime_to_quaint() {
        let column_type = ColumnType::DateTime;

        // null
        test_null(QuaintValueType::DateTime(None).into(), column_type);

        let s = "2023-01-01 23:59:59.415";
        let json_value = serde_json::Value::String(s.to_string());
        let quaint_value = js_value_to_quaint(json_value, column_type, "column_name").unwrap();

        let datetime = NaiveDate::from_ymd_opt(2023, 1, 1)
            .unwrap()
            .and_hms_milli_opt(23, 59, 59, 415)
            .unwrap();
        let datetime = DateTime::from_utc(datetime, Utc);
        assert_eq!(quaint_value.inner, QuaintValueKind::DateTime(Some(datetime)));

        let s = "2023-01-01 23:59:59.123456";
        let json_value = serde_json::Value::String(s.to_string());
        let quaint_value = js_value_to_quaint(json_value, column_type, "column_name").unwrap();

        let datetime = NaiveDate::from_ymd_opt(2023, 1, 1)
            .unwrap()
            .and_hms_micro_opt(23, 59, 59, 123_456)
            .unwrap();
        let datetime = DateTime::from_utc(datetime, Utc);
        assert_eq!(quaint_value.inner, QuaintValueKind::DateTime(Some(datetime)));

        let s = "2023-01-01 23:59:59";
        let json_value = serde_json::Value::String(s.to_string());
        let quaint_value = js_value_to_quaint(json_value, column_type, "column_name").unwrap();

        let datetime = NaiveDate::from_ymd_opt(2023, 1, 1)
            .unwrap()
            .and_hms_milli_opt(23, 59, 59, 0)
            .unwrap();
        let datetime = DateTime::from_utc(datetime, Utc);
        assert_eq!(quaint_value.inner, QuaintValueKind::DateTime(Some(datetime)));
    }

    #[test]
    fn js_value_json_to_quaint() {
        let column_type = ColumnType::Json;

        // null
        test_null(QuaintValueType::Json(None).into(), column_type);

        let json = json!({
            "key": "value",
            "nested": [
                true,
                false,
                1,
                null
            ]
        });
        let json_value = json.clone();
        let quaint_value = js_value_to_quaint(json_value, column_type, "column_name").unwrap();
        assert_eq!(quaint_value.inner, QuaintValueKind::Json(Some(json.clone())));
    }

    #[test]
    fn js_value_enum_to_quaint() {
        let column_type = ColumnType::Enum;

        // null
        test_null(QuaintValue::Enum(None, None), column_type);

        let s = "some enum variant";
        let json_value = serde_json::Value::String(s.to_string());

        let quaint_value = js_value_to_quaint(json_value, column_type, "column_name").unwrap();
        assert_eq!(quaint_value.inner, QuaintValueKind::Enum(Some(s.into()), None));
    }

    #[test]
    fn js_int32_array_to_quaint() {
        let column_type = ColumnType::Int32Array;
        test_null(QuaintValue::Array(None), column_type);

        let json_value = json!([1, 2, 3]);
        let quaint_value = js_value_to_quaint(json_value, column_type, "column_name").unwrap();

        assert_eq!(
            quaint_value,
            QuaintValue::Array(Some(vec![
                QuaintValue::int32(1),
                QuaintValue::int32(2),
                QuaintValue::int32(3)
            ]))
        );

        let json_value = json!([1, 2, {}]);
        let quaint_value = js_value_to_quaint(json_value, column_type, "column_name");

        assert_eq!(
            quaint_value.err().unwrap().to_string(),
            "Conversion failed: expected an i32 number in column column_name[2], found {}"
        );
    }

    #[test]
    fn js_text_array_to_quaint() {
        let column_type = ColumnType::TextArray;
        test_null(QuaintValue::Array(None), column_type);

        let json_value = json!(["hi", "there"]);
        let quaint_value = js_value_to_quaint(json_value, column_type, "column_name").unwrap();

        assert_eq!(
            quaint_value,
            QuaintValue::Array(Some(vec![QuaintValue::text("hi"), QuaintValue::text("there"),]))
        );

        let json_value = json!([10]);
        let quaint_value = js_value_to_quaint(json_value, column_type, "column_name");

        assert_eq!(
            quaint_value.err().unwrap().to_string(),
            "Conversion failed: expected a string in column column_name[0], found 10"
        );
    }
}
