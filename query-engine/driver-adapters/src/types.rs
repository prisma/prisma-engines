// `clippy::empty_docs` is required because of the `tsify` crate.
#![allow(unused_imports, clippy::empty_docs)]

use std::str::FromStr;

#[cfg(not(target_arch = "wasm32"))]
use napi::bindgen_prelude::{FromNapiValue, ToNapiValue};

use quaint::connector::{ColumnType as QuaintColumnType, ExternalConnectionInfo, SqlFamily};
#[cfg(target_arch = "wasm32")]
use tsify::Tsify;

use crate::conversion::{JSArg, JSArgType};
use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};

#[cfg_attr(target_arch = "wasm32", derive(Deserialize))]
#[derive(Debug, Eq, PartialEq, Clone)]
pub enum AdapterFlavour {
    #[cfg(feature = "mysql")]
    Mysql,
    #[cfg(feature = "postgresql")]
    Postgres,
    #[cfg(feature = "sqlite")]
    Sqlite,
}

impl AdapterFlavour {
    pub fn db_system_name(&self) -> &'static str {
        match self {
            Self::Mysql => "mysql",
            Self::Postgres => "postgresql",
            Self::Sqlite => "sqlite",
        }
    }
}

impl FromStr for AdapterFlavour {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            #[cfg(feature = "postgresql")]
            "postgres" => Ok(Self::Postgres),
            #[cfg(feature = "mysql")]
            "mysql" => Ok(Self::Mysql),
            #[cfg(feature = "sqlite")]
            "sqlite" => Ok(Self::Sqlite),
            _ => Err(format!("Unsupported adapter flavour: {:?}", s)),
        }
    }
}

impl From<&AdapterFlavour> for SqlFamily {
    fn from(value: &AdapterFlavour) -> Self {
        match value {
            #[cfg(feature = "mysql")]
            AdapterFlavour::Mysql => SqlFamily::Mysql,
            #[cfg(feature = "postgresql")]
            AdapterFlavour::Postgres => SqlFamily::Postgres,
            #[cfg(feature = "sqlite")]
            AdapterFlavour::Sqlite => SqlFamily::Sqlite,
        }
    }
}

#[cfg_attr(not(target_arch = "wasm32"), napi_derive::napi(object))]
#[cfg_attr(target_arch = "wasm32", derive(Deserialize))]
#[cfg_attr(target_arch = "wasm32", serde(rename_all = "camelCase"))]
#[derive(Default)]
pub(crate) struct JsConnectionInfo {
    pub schema_name: Option<String>,
    pub max_bind_values: Option<u32>,
}

impl JsConnectionInfo {
    pub fn into_external_connection_info(self, provider: &AdapterFlavour) -> ExternalConnectionInfo {
        let schema_name = self.get_schema_name(provider);
        let sql_family = SqlFamily::from(provider);

        ExternalConnectionInfo::new(
            sql_family,
            schema_name.to_owned(),
            self.max_bind_values.map(|v| v as usize),
        )
    }

    fn get_schema_name(&self, provider: &AdapterFlavour) -> &str {
        match self.schema_name.as_ref() {
            Some(name) => name,
            None => self.default_schema_name(provider),
        }
    }

    fn default_schema_name(&self, provider: &AdapterFlavour) -> &str {
        match provider {
            #[cfg(feature = "mysql")]
            AdapterFlavour::Mysql => quaint::connector::DEFAULT_MYSQL_DB,
            #[cfg(feature = "postgresql")]
            AdapterFlavour::Postgres => quaint::connector::DEFAULT_POSTGRES_SCHEMA,
            #[cfg(feature = "sqlite")]
            AdapterFlavour::Sqlite => quaint::connector::DEFAULT_SQLITE_DATABASE,
        }
    }
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
#[cfg_attr(not(target_arch = "wasm32"), napi_derive::napi(object))]
#[cfg_attr(target_arch = "wasm32", derive(Deserialize))]
#[cfg_attr(target_arch = "wasm32", serde(rename_all = "camelCase"))]
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

macro_rules! js_column_type {
    ($($(#[$($attrss:tt)*])*$name:ident($val:expr) => $quaint_name:ident,)*) => {
        #[cfg_attr(not(target_arch = "wasm32"), napi_derive::napi)]
        #[cfg_attr(target_arch = "wasm32", derive(Clone, Copy, Deserialize_repr))]
        #[repr(u8)]
        #[derive(Debug)]
        pub enum ColumnType {
            $(
                $(#[$($attrss)*])*
                $name = $val,
            )*
        }

        impl From<&ColumnType> for QuaintColumnType {
            fn from(value: &ColumnType) -> Self {
                match value {
                    $(ColumnType::$name => QuaintColumnType::$quaint_name,)*
                }
            }
        }
    };
}

// JsColumnType(discriminant) => quaint::ColumnType
js_column_type! {
    /// [PLANETSCALE_TYPE] (MYSQL_TYPE) -> [TypeScript example]
    /// The following PlanetScale type IDs are mapped into Int32:
    /// - INT8 (TINYINT) -> e.g. `127`
    /// - INT16 (SMALLINT) -> e.g. `32767`
    /// - INT24 (MEDIUMINT) -> e.g. `8388607`
    /// - INT32 (INT) -> e.g. `2147483647`
    Int32(0) => Int32,

    /// The following PlanetScale type IDs are mapped into Int64:
    /// - INT64 (BIGINT) -> e.g. `"9223372036854775807"` (String-encoded)
    Int64(1) => Int64,

    /// The following PlanetScale type IDs are mapped into Float:
    /// - FLOAT32 (FLOAT) -> e.g. `3.402823466`
    Float(2) => Float,

    /// The following PlanetScale type IDs are mapped into Double:
    /// - FLOAT64 (DOUBLE) -> e.g. `1.7976931348623157`
    Double(3) => Double,

    /// The following PlanetScale type IDs are mapped into Numeric:
    /// - DECIMAL (DECIMAL) -> e.g. `"99999999.99"` (String-encoded)
    Numeric(4) => Numeric,

    /// The following PlanetScale type IDs are mapped into Boolean:
    /// - BOOLEAN (BOOLEAN) -> e.g. `1`
    Boolean(5) => Boolean,


    Character(6) => Char,

    /// The following PlanetScale type IDs are mapped into Text:
    /// - TEXT (TEXT) -> e.g. `"foo"` (String-encoded)
    /// - VARCHAR (VARCHAR) -> e.g. `"foo"` (String-encoded)
    Text(7) => Text,

    /// The following PlanetScale type IDs are mapped into Date:
    /// - DATE (DATE) -> e.g. `"2023-01-01"` (String-encoded, yyyy-MM-dd)
    Date(8) => Date,

    /// The following PlanetScale type IDs are mapped into Time:
    /// - TIME (TIME) -> e.g. `"23:59:59"` (String-encoded, HH:mm:ss)
    Time(9) => Time,


    /// The following PlanetScale type IDs are mapped into DateTime:
    /// - DATETIME (DATETIME) -> e.g. `"2023-01-01 23:59:59"` (String-encoded, yyyy-MM-dd HH:mm:ss)
    /// - TIMESTAMP (TIMESTAMP) -> e.g. `"2023-01-01 23:59:59"` (String-encoded, yyyy-MM-dd HH:mm:ss)
    DateTime(10) => DateTime,

    /// The following PlanetScale type IDs are mapped into Json:
    /// - JSON (JSON) -> e.g. `"{\"key\": \"value\"}"` (String-encoded)
    Json(11) => Json,

    /// The following PlanetScale type IDs are mapped into Enum:
    /// - ENUM (ENUM) -> e.g. `"foo"` (String-encoded)
    Enum(12) => Enum,


    /// The following PlanetScale type IDs are mapped into Bytes:
    /// - BLOB (BLOB) -> e.g. `"\u0012"` (String-encoded)
    /// - VARBINARY (VARBINARY) -> e.g. `"\u0012"` (String-encoded)
    /// - BINARY (BINARY) -> e.g. `"\u0012"` (String-encoded)
    /// - GEOMETRY (GEOMETRY) -> e.g. `"\u0012"` (String-encoded)
    Bytes(13) => Bytes,


    /// The following PlanetScale type IDs are mapped into Set:
    /// - SET (SET) -> e.g. `"foo,bar"` (String-encoded, comma-separated)
    /// This is currently unhandled, and will panic if encountered.
    Set(14) => Text,

    /// UUID from postgres-flavored driver adapters is mapped to this type.
    Uuid(15) => Uuid,

    /// Int32 array (INT2_ARRAY and INT4_ARRAY in PostgreSQL)
    Int32Array(64) => Int32Array,

    /// Int64 array (INT8_ARRAY in PostgreSQL)
    Int64Array(65) => Int64Array,

    /// Float array (FLOAT4_ARRAY in PostgreSQL)
    FloatArray(66) => FloatArray,

    /// Double array (FLOAT8_ARRAY in PostgreSQL)
    DoubleArray(67) => DoubleArray,

    /// Numeric array (NUMERIC_ARRAY, MONEY_ARRAY etc in PostgreSQL)
    NumericArray(68) => NumericArray,

    /// Boolean array (BOOL_ARRAY in PostgreSQL)
    BooleanArray(69) => BooleanArray,

    /// Char array (CHAR_ARRAY in PostgreSQL)
    CharacterArray(70) => CharArray,

    /// Text array (TEXT_ARRAY in PostgreSQL)
    TextArray(71) => TextArray,

    /// Date array (DATE_ARRAY in PostgreSQL)
    DateArray(72) => DateArray,

    /// Time array (TIME_ARRAY in PostgreSQL)
    TimeArray(73) => TimeArray,

    /// DateTime array (TIMESTAMP_ARRAY in PostgreSQL)
    DateTimeArray(74) => DateTimeArray,

    /// Json array (JSON_ARRAY in PostgreSQL)
    JsonArray(75) => JsonArray,

    /// Enum array (ENUM_ARRAY in PostgreSQL)
    EnumArray(76) => TextArray,

    /// Bytes array (BYTEA_ARRAY in PostgreSQL)
    BytesArray(77) => BytesArray,

    /// Uuid array (UUID_ARRAY in PostgreSQL)
    UuidArray(78) => UuidArray,

    /*
     * Below there are custom types that don't have a 1:1 translation with a quaint::Value.
     * enum variant.
     */
    /// UnknownNumber is used when the type of the column is a number but of unknown particular type
    /// and precision.
    ///
    /// It's used by some driver adapters, like libsql to return aggregation values like AVG, or
    /// COUNT, and it can be mapped to either Int64, or Double
    UnknownNumber(128) => Unknown,
}

#[cfg_attr(not(target_arch = "wasm32"), napi_derive::napi(object))]
#[derive(Debug, Default)]
pub struct Query {
    pub sql: String,
    pub args: Vec<JSArg>,
    pub arg_types: Vec<JSArgType>,
}

#[cfg_attr(not(target_arch = "wasm32"), napi_derive::napi(object))]
#[cfg_attr(target_arch = "wasm32", derive(Deserialize, Tsify))]
#[cfg_attr(target_arch = "wasm32", serde(rename_all = "camelCase"))]
#[derive(Debug, Default)]
pub struct TransactionOptions {
    /// Whether or not to run a phantom query (i.e., a query that only influences Prisma event logs, but not the database itself)
    /// before opening a transaction, committing, or rollbacking.
    pub use_phantom_query: bool,
}
