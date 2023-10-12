use crate::ast::*;
use crate::error::{Error, ErrorKind};

use bigdecimal::{BigDecimal, FromPrimitive, ToPrimitive};
use chrono::{DateTime, NaiveDate, NaiveTime, Utc};
use serde_json::{Number, Value as JsonValue};
use std::fmt::Display;
use std::{
    borrow::{Borrow, Cow},
    convert::TryFrom,
    fmt,
    str::FromStr,
};
use uuid::Uuid;

/// A value written to the query as-is without parameterization.
#[derive(Debug, Clone, PartialEq)]
pub struct Raw<'a>(pub(crate) Value<'a>);

/// Converts the value into a state to skip parameterization.
///
/// Must be used carefully to avoid SQL injections.
pub trait IntoRaw<'a> {
    fn raw(self) -> Raw<'a>;
}

impl<'a, T> IntoRaw<'a> for T
where
    T: Into<Value<'a>>,
{
    fn raw(self) -> Raw<'a> {
        Raw(self.into())
    }
}

/// A native-column type, i.e. the connector-specific type of the column.
#[derive(Debug, Clone, PartialEq)]
pub struct NativeColumnType<'a>(Cow<'a, str>);

impl<'a> std::ops::Deref for NativeColumnType<'a> {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'a> From<&'a str> for NativeColumnType<'a> {
    fn from(s: &'a str) -> Self {
        Self(Cow::Owned(s.to_uppercase()))
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Value<'a> {
    pub typed: ValueType<'a>,
    pub native_column_type: Option<NativeColumnType<'a>>,
}

impl<'a> Value<'a> {
    /// Returns the native column type of the value, if any, in the form
    /// of an UPCASE string. ex: "VARCHAR, BYTEA, DATE, TIMEZ"  
    pub fn native_column_type_name(&'a self) -> Option<&'a str> {
        self.native_column_type.as_deref()
    }

    /// Changes the value to include information about the native column type
    pub fn with_native_column_type<T: Into<NativeColumnType<'a>>>(mut self, column_type: Option<T>) -> Self {
        self.native_column_type = column_type.map(|ct| ct.into());
        self
    }

    /// Creates a new 32-bit signed integer.
    pub fn int32<I>(value: I) -> Self
    where
        I: Into<i32>,
    {
        ValueType::int32(value).into_value()
    }

    /// Creates a new 64-bit signed integer.
    pub fn int64<I>(value: I) -> Self
    where
        I: Into<i64>,
    {
        ValueType::int64(value).into_value()
    }

    /// Creates a new decimal value.
    pub fn numeric(value: BigDecimal) -> Self {
        ValueType::numeric(value).into_value()
    }

    /// Creates a new float value.
    pub fn float(value: f32) -> Self {
        ValueType::float(value).into_value()
    }

    /// Creates a new double value.
    pub fn double(value: f64) -> Self {
        ValueType::double(value).into_value()
    }

    /// Creates a new string value.
    pub fn text<T>(value: T) -> Self
    where
        T: Into<Cow<'a, str>>,
    {
        ValueType::text(value).into_value()
    }

    /// Creates a new enum value.
    pub fn enum_variant<T>(value: T) -> Self
    where
        T: Into<EnumVariant<'a>>,
    {
        ValueType::enum_variant(value).into_value()
    }

    /// Creates a new enum value with the name of the enum attached.
    pub fn enum_variant_with_name<T, U>(value: T, name: U) -> Self
    where
        T: Into<EnumVariant<'a>>,
        U: Into<EnumName<'a>>,
    {
        ValueType::enum_variant_with_name(value, name).into_value()
    }

    /// Creates a new enum array value
    pub fn enum_array<T>(value: T) -> Self
    where
        T: IntoIterator<Item = EnumVariant<'a>>,
    {
        ValueType::enum_array(value).into_value()
    }

    /// Creates a new enum array value with the name of the enum attached.
    pub fn enum_array_with_name<T, U>(value: T, name: U) -> Self
    where
        T: IntoIterator<Item = EnumVariant<'a>>,
        U: Into<EnumName<'a>>,
    {
        ValueType::enum_array_with_name(value, name).into_value()
    }

    /// Creates a new bytes value.
    pub fn bytes<B>(value: B) -> Self
    where
        B: Into<Cow<'a, [u8]>>,
    {
        ValueType::bytes(value).into_value()
    }

    /// Creates a new boolean value.
    pub fn boolean<B>(value: B) -> Self
    where
        B: Into<bool>,
    {
        ValueType::boolean(value).into_value()
    }

    /// Creates a new character value.
    pub fn character<C>(value: C) -> Self
    where
        C: Into<char>,
    {
        ValueType::character(value).into_value()
    }

    /// Creates a new array value.
    pub fn array<I, V>(value: I) -> Self
    where
        I: IntoIterator<Item = V>,
        V: Into<Value<'a>>,
    {
        ValueType::array(value).into_value()
    }

    /// Creates a new uuid value.
    pub fn uuid(value: Uuid) -> Self {
        ValueType::uuid(value).into_value()
    }

    /// Creates a new datetime value.
    pub fn datetime(value: DateTime<Utc>) -> Self {
        ValueType::datetime(value).into_value()
    }

    /// Creates a new date value.
    pub fn date(value: NaiveDate) -> Self {
        ValueType::date(value).into_value()
    }

    /// Creates a new time value.
    pub fn time(value: NaiveTime) -> Self {
        ValueType::time(value).into_value()
    }

    /// Creates a new JSON value.
    pub fn json(value: serde_json::Value) -> Self {
        ValueType::json(value).into_value()
    }

    /// Creates a new XML value.
    pub fn xml<T>(value: T) -> Self
    where
        T: Into<Cow<'a, str>>,
    {
        ValueType::xml(value).into_value()
    }

    /// `true` if the `Value` is null.
    pub fn is_null(&self) -> bool {
        self.typed.is_null()
    }

    /// Returns a &str if the value is text, otherwise `None`.
    pub fn as_str(&self) -> Option<&str> {
        self.typed.as_str()
    }

    /// `true` if the `Value` is text.
    pub fn is_text(&self) -> bool {
        self.typed.is_text()
    }

    /// Returns a char if the value is a char, otherwise `None`.
    pub fn as_char(&self) -> Option<char> {
        self.typed.as_char()
    }

    /// Returns a cloned String if the value is text, otherwise `None`.
    pub fn to_string(&self) -> Option<String> {
        self.typed.to_string()
    }

    /// Transforms the `Value` to a `String` if it's text,
    /// otherwise `None`.
    pub fn into_string(self) -> Option<String> {
        self.typed.into_string()
    }

    /// Returns whether this value is the `Bytes` variant.
    pub fn is_bytes(&self) -> bool {
        self.typed.is_bytes()
    }

    /// Returns a bytes slice if the value is text or a byte slice, otherwise `None`.
    pub fn as_bytes(&self) -> Option<&[u8]> {
        self.typed.as_bytes()
    }

    /// Returns a cloned `Vec<u8>` if the value is text or a byte slice, otherwise `None`.
    pub fn to_bytes(&self) -> Option<Vec<u8>> {
        self.typed.to_bytes()
    }

    /// `true` if the `Value` is a 32-bit signed integer.
    pub fn is_i32(&self) -> bool {
        self.typed.is_i32()
    }

    /// `true` if the `Value` is a 64-bit signed integer.
    pub fn is_i64(&self) -> bool {
        self.typed.is_i64()
    }

    /// `true` if the `Value` is a signed integer.
    pub fn is_integer(&self) -> bool {
        self.typed.is_integer()
    }

    /// Returns an `i64` if the value is a 64-bit signed integer, otherwise `None`.
    pub fn as_i64(&self) -> Option<i64> {
        self.typed.as_i64()
    }

    /// Returns an `i32` if the value is a 32-bit signed integer, otherwise `None`.
    pub fn as_i32(&self) -> Option<i32> {
        self.typed.as_i32()
    }

    /// Returns an `i64` if the value is a signed integer, otherwise `None`.
    pub fn as_integer(&self) -> Option<i64> {
        self.typed.as_integer()
    }

    /// Returns a `f64` if the value is a double, otherwise `None`.
    pub fn as_f64(&self) -> Option<f64> {
        self.typed.as_f64()
    }

    /// Returns a `f32` if the value is a double, otherwise `None`.
    pub fn as_f32(&self) -> Option<f32> {
        self.typed.as_f32()
    }

    /// `true` if the `Value` is a numeric value or can be converted to one.

    pub fn is_numeric(&self) -> bool {
        self.typed.is_numeric()
    }

    /// Returns a bigdecimal, if the value is a numeric, float or double value,
    /// otherwise `None`.

    pub fn into_numeric(self) -> Option<BigDecimal> {
        self.typed.into_numeric()
    }

    /// Returns a reference to a bigdecimal, if the value is a numeric.
    /// Otherwise `None`.

    pub fn as_numeric(&self) -> Option<&BigDecimal> {
        self.typed.as_numeric()
    }

    /// `true` if the `Value` is a boolean value.
    pub fn is_bool(&self) -> bool {
        self.typed.is_bool()
    }

    /// Returns a bool if the value is a boolean, otherwise `None`.
    pub fn as_bool(&self) -> Option<bool> {
        self.typed.as_bool()
    }

    /// `true` if the `Value` is an Array.
    pub fn is_array(&self) -> bool {
        self.typed.is_array()
    }

    /// `true` if the `Value` is of UUID type.
    pub fn is_uuid(&self) -> bool {
        self.typed.is_uuid()
    }

    /// Returns an UUID if the value is of UUID type, otherwise `None`.
    pub fn as_uuid(&self) -> Option<Uuid> {
        self.typed.as_uuid()
    }

    /// `true` if the `Value` is a DateTime.
    pub fn is_datetime(&self) -> bool {
        self.typed.is_datetime()
    }

    /// Returns a `DateTime` if the value is a `DateTime`, otherwise `None`.
    pub fn as_datetime(&self) -> Option<DateTime<Utc>> {
        self.typed.as_datetime()
    }

    /// `true` if the `Value` is a Date.
    pub fn is_date(&self) -> bool {
        self.typed.is_date()
    }

    /// Returns a `NaiveDate` if the value is a `Date`, otherwise `None`.
    pub fn as_date(&self) -> Option<NaiveDate> {
        self.typed.as_date()
    }

    /// `true` if the `Value` is a `Time`.
    pub fn is_time(&self) -> bool {
        self.typed.is_time()
    }

    /// Returns a `NaiveTime` if the value is a `Time`, otherwise `None`.
    pub fn as_time(&self) -> Option<NaiveTime> {
        self.typed.as_time()
    }

    /// `true` if the `Value` is a JSON value.
    pub fn is_json(&self) -> bool {
        self.typed.is_json()
    }

    /// Returns a reference to a JSON Value if of Json type, otherwise `None`.
    pub fn as_json(&self) -> Option<&serde_json::Value> {
        self.typed.as_json()
    }

    /// Transforms to a JSON Value if of Json type, otherwise `None`.
    pub fn into_json(self) -> Option<serde_json::Value> {
        self.typed.into_json()
    }

    /// Returns a `Vec<T>` if the value is an array of `T`, otherwise `None`.
    pub fn into_vec<T>(self) -> Option<Vec<T>>
    where
        T: TryFrom<Value<'a>>,
    {
        self.typed.into_vec()
    }

    /// Returns a cloned Vec<T> if the value is an array of T, otherwise `None`.
    pub fn to_vec<T>(&self) -> Option<Vec<T>>
    where
        T: TryFrom<Value<'a>>,
    {
        self.typed.to_vec()
    }

    pub fn null_int32() -> Self {
        ValueType::Int32(None).into()
    }

    pub fn null_int64() -> Self {
        ValueType::Int64(None).into()
    }

    pub fn null_float() -> Self {
        ValueType::Float(None).into()
    }

    pub fn null_double() -> Self {
        ValueType::Double(None).into()
    }

    pub fn null_text() -> Self {
        ValueType::Text(None).into()
    }

    pub fn null_enum() -> Self {
        ValueType::Enum(None, None).into()
    }

    pub fn null_enum_array() -> Self {
        ValueType::EnumArray(None, None).into()
    }

    pub fn null_bytes() -> Self {
        ValueType::Bytes(None).into()
    }

    pub fn null_boolean() -> Self {
        ValueType::Boolean(None).into()
    }

    pub fn null_character() -> Self {
        ValueType::Char(None).into()
    }

    pub fn null_array() -> Self {
        ValueType::Array(None).into()
    }

    pub fn null_numeric() -> Self {
        ValueType::Numeric(None).into()
    }

    pub fn null_json() -> Self {
        ValueType::Json(None).into()
    }

    pub fn null_xml() -> Self {
        ValueType::Xml(None).into()
    }

    pub fn null_uuid() -> Self {
        ValueType::Uuid(None).into()
    }

    pub fn null_datetime() -> Self {
        ValueType::DateTime(None).into()
    }

    pub fn null_date() -> Self {
        ValueType::Date(None).into()
    }

    pub fn null_time() -> Self {
        ValueType::Time(None).into()
    }
}

impl<'a> Display for Value<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.typed.fmt(f)
    }
}

impl<'a> From<ValueType<'a>> for Value<'a> {
    fn from(inner: ValueType<'a>) -> Self {
        Self {
            typed: inner,
            native_column_type: Default::default(),
        }
    }
}

impl<'a> From<Value<'a>> for ValueType<'a> {
    fn from(val: Value<'a>) -> Self {
        val.typed
    }
}

/// A value we must parameterize for the prepared statement. Null values should be
/// defined by their corresponding type variants with a `None` value for best
/// compatibility.
#[derive(Debug, Clone, PartialEq)]
pub enum ValueType<'a> {
    /// 32-bit signed integer.
    Int32(Option<i32>),
    /// 64-bit signed integer.
    Int64(Option<i64>),
    /// 32-bit floating point.
    Float(Option<f32>),
    /// 64-bit floating point.
    Double(Option<f64>),
    /// String value.
    Text(Option<Cow<'a, str>>),
    /// Database enum value.
    /// The optional `EnumName` is only used on PostgreSQL.
    /// Read more about it here: https://github.com/prisma/prisma-engines/pull/4280
    Enum(Option<EnumVariant<'a>>, Option<EnumName<'a>>),
    /// Database enum array (PostgreSQL specific).
    /// We use a different variant than `ValueType::Array` to uplift the `EnumName`
    /// and have it available even for empty enum arrays.
    EnumArray(Option<Vec<EnumVariant<'a>>>, Option<EnumName<'a>>),
    /// Bytes value.
    Bytes(Option<Cow<'a, [u8]>>),
    /// Boolean value.
    Boolean(Option<bool>),
    /// A single character.
    Char(Option<char>),
    /// An array value (PostgreSQL).
    Array(Option<Vec<Value<'a>>>),
    /// A numeric value.
    Numeric(Option<BigDecimal>),
    /// A JSON value.
    Json(Option<serde_json::Value>),
    /// A XML value.
    Xml(Option<Cow<'a, str>>),
    /// An UUID value.
    Uuid(Option<Uuid>),
    /// A datetime value.
    DateTime(Option<DateTime<Utc>>),
    /// A date value.
    Date(Option<NaiveDate>),
    /// A time value.
    Time(Option<NaiveTime>),
}

pub(crate) struct Params<'a>(pub(crate) &'a [Value<'a>]);

impl<'a> Display for Params<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let len = self.0.len();

        write!(f, "[")?;
        for (i, val) in self.0.iter().enumerate() {
            write!(f, "{val}")?;

            if i < (len - 1) {
                write!(f, ",")?;
            }
        }
        write!(f, "]")
    }
}

impl<'a> fmt::Display for ValueType<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let res = match self {
            ValueType::Int32(val) => val.map(|v| write!(f, "{v}")),
            ValueType::Int64(val) => val.map(|v| write!(f, "{v}")),
            ValueType::Float(val) => val.map(|v| write!(f, "{v}")),
            ValueType::Double(val) => val.map(|v| write!(f, "{v}")),
            ValueType::Text(val) => val.as_ref().map(|v| write!(f, "\"{v}\"")),
            ValueType::Bytes(val) => val.as_ref().map(|v| write!(f, "<{} bytes blob>", v.len())),
            ValueType::Enum(val, _) => val.as_ref().map(|v| write!(f, "\"{v}\"")),
            ValueType::EnumArray(vals, _) => vals.as_ref().map(|vals| {
                let len = vals.len();

                write!(f, "[")?;
                for (i, val) in vals.iter().enumerate() {
                    write!(f, "{val}")?;

                    if i < (len - 1) {
                        write!(f, ",")?;
                    }
                }
                write!(f, "]")
            }),
            ValueType::Boolean(val) => val.map(|v| write!(f, "{v}")),
            ValueType::Char(val) => val.map(|v| write!(f, "'{v}'")),
            ValueType::Array(vals) => vals.as_ref().map(|vals| {
                let len = vals.len();

                write!(f, "[")?;
                for (i, val) in vals.iter().enumerate() {
                    write!(f, "{val}")?;

                    if i < (len - 1) {
                        write!(f, ",")?;
                    }
                }
                write!(f, "]")
            }),
            ValueType::Xml(val) => val.as_ref().map(|v| write!(f, "{v}")),

            ValueType::Numeric(val) => val.as_ref().map(|v| write!(f, "{v}")),
            ValueType::Json(val) => val.as_ref().map(|v| write!(f, "{v}")),
            ValueType::Uuid(val) => val.map(|v| write!(f, "\"{v}\"")),
            ValueType::DateTime(val) => val.map(|v| write!(f, "\"{v}\"")),
            ValueType::Date(val) => val.map(|v| write!(f, "\"{v}\"")),
            ValueType::Time(val) => val.map(|v| write!(f, "\"{v}\"")),
        };

        match res {
            Some(r) => r,
            None => write!(f, "null"),
        }
    }
}

impl<'a> From<Value<'a>> for serde_json::Value {
    fn from(pv: Value<'a>) -> Self {
        pv.typed.into()
    }
}

impl<'a> From<ValueType<'a>> for serde_json::Value {
    fn from(pv: ValueType<'a>) -> Self {
        let res = match pv {
            ValueType::Int32(i) => i.map(|i| serde_json::Value::Number(Number::from(i))),
            ValueType::Int64(i) => i.map(|i| serde_json::Value::Number(Number::from(i))),
            ValueType::Float(f) => f.map(|f| match Number::from_f64(f as f64) {
                Some(number) => serde_json::Value::Number(number),
                None => serde_json::Value::Null,
            }),
            ValueType::Double(f) => f.map(|f| match Number::from_f64(f) {
                Some(number) => serde_json::Value::Number(number),
                None => serde_json::Value::Null,
            }),
            ValueType::Text(cow) => cow.map(|cow| serde_json::Value::String(cow.into_owned())),
            ValueType::Bytes(bytes) => bytes.map(|bytes| serde_json::Value::String(base64::encode(bytes))),
            ValueType::Enum(cow, _) => cow.map(|cow| serde_json::Value::String(cow.into_owned())),
            ValueType::EnumArray(values, _) => values.map(|values| {
                serde_json::Value::Array(
                    values
                        .into_iter()
                        .map(|value| serde_json::Value::String(value.into_owned()))
                        .collect(),
                )
            }),
            ValueType::Boolean(b) => b.map(serde_json::Value::Bool),
            ValueType::Char(c) => c.map(|c| {
                let bytes = [c as u8];
                let s = std::str::from_utf8(&bytes)
                    .expect("interpret byte as UTF-8")
                    .to_string();
                serde_json::Value::String(s)
            }),
            ValueType::Xml(cow) => cow.map(|cow| serde_json::Value::String(cow.into_owned())),
            ValueType::Array(v) => {
                v.map(|v| serde_json::Value::Array(v.into_iter().map(serde_json::Value::from).collect()))
            }

            ValueType::Numeric(d) => d.map(|d| serde_json::to_value(d.to_f64().unwrap()).unwrap()),
            ValueType::Json(v) => v,
            ValueType::Uuid(u) => u.map(|u| serde_json::Value::String(u.hyphenated().to_string())),
            ValueType::DateTime(dt) => dt.map(|dt| serde_json::Value::String(dt.to_rfc3339())),
            ValueType::Date(date) => date.map(|date| serde_json::Value::String(format!("{date}"))),
            ValueType::Time(time) => time.map(|time| serde_json::Value::String(format!("{time}"))),
        };

        match res {
            Some(val) => val,
            None => serde_json::Value::Null,
        }
    }
}

impl<'a> ValueType<'a> {
    pub fn into_value(self) -> Value<'a> {
        self.into()
    }

    /// Creates a new 32-bit signed integer.
    pub(crate) fn int32<I>(value: I) -> Self
    where
        I: Into<i32>,
    {
        Self::Int32(Some(value.into()))
    }

    /// Creates a new 64-bit signed integer.
    pub(crate) fn int64<I>(value: I) -> Self
    where
        I: Into<i64>,
    {
        Self::Int64(Some(value.into()))
    }

    /// Creates a new decimal value.

    pub(crate) fn numeric(value: BigDecimal) -> Self {
        Self::Numeric(Some(value))
    }

    /// Creates a new float value.
    pub(crate) fn float(value: f32) -> Self {
        Self::Float(Some(value))
    }

    /// Creates a new double value.
    pub(crate) fn double(value: f64) -> Self {
        Self::Double(Some(value))
    }

    /// Creates a new string value.
    pub(crate) fn text<T>(value: T) -> Self
    where
        T: Into<Cow<'a, str>>,
    {
        Self::Text(Some(value.into()))
    }

    /// Creates a new enum value.
    pub(crate) fn enum_variant<T>(value: T) -> Self
    where
        T: Into<EnumVariant<'a>>,
    {
        Self::Enum(Some(value.into()), None)
    }

    /// Creates a new enum value with the name of the enum attached.
    pub(crate) fn enum_variant_with_name<T, U>(value: T, enum_name: U) -> Self
    where
        T: Into<EnumVariant<'a>>,
        U: Into<EnumName<'a>>,
    {
        Self::Enum(Some(value.into()), Some(enum_name.into()))
    }

    /// Creates a new enum array value
    pub(crate) fn enum_array<T>(value: T) -> Self
    where
        T: IntoIterator<Item = EnumVariant<'a>>,
    {
        Self::EnumArray(Some(value.into_iter().collect()), None)
    }

    /// Creates a new enum array value with the name of the enum attached.
    pub(crate) fn enum_array_with_name<T, U>(value: T, name: U) -> Self
    where
        T: IntoIterator<Item = EnumVariant<'a>>,
        U: Into<EnumName<'a>>,
    {
        Self::EnumArray(Some(value.into_iter().collect()), Some(name.into()))
    }

    /// Creates a new bytes value.
    pub(crate) fn bytes<B>(value: B) -> Self
    where
        B: Into<Cow<'a, [u8]>>,
    {
        Self::Bytes(Some(value.into()))
    }

    /// Creates a new boolean value.
    pub(crate) fn boolean<B>(value: B) -> Self
    where
        B: Into<bool>,
    {
        Self::Boolean(Some(value.into()))
    }

    /// Creates a new character value.
    pub(crate) fn character<C>(value: C) -> Self
    where
        C: Into<char>,
    {
        Self::Char(Some(value.into()))
    }

    /// Creates a new array value.
    pub(crate) fn array<I, V>(value: I) -> Self
    where
        I: IntoIterator<Item = V>,
        V: Into<Value<'a>>,
    {
        Self::Array(Some(value.into_iter().map(|v| v.into()).collect()))
    }

    /// Creates a new uuid value.
    pub(crate) fn uuid(value: Uuid) -> Self {
        Self::Uuid(Some(value))
    }

    /// Creates a new datetime value.
    pub(crate) fn datetime(value: DateTime<Utc>) -> Self {
        Self::DateTime(Some(value))
    }

    /// Creates a new date value.
    pub(crate) fn date(value: NaiveDate) -> Self {
        Self::Date(Some(value))
    }

    /// Creates a new time value.
    pub(crate) fn time(value: NaiveTime) -> Self {
        Self::Time(Some(value))
    }

    /// Creates a new JSON value.
    pub(crate) fn json(value: serde_json::Value) -> Self {
        Self::Json(Some(value))
    }

    /// Creates a new XML value.
    pub(crate) fn xml<T>(value: T) -> Self
    where
        T: Into<Cow<'a, str>>,
    {
        Self::Xml(Some(value.into()))
    }

    /// `true` if the `Value` is null.
    pub fn is_null(&self) -> bool {
        match self {
            Self::Int32(i) => i.is_none(),
            Self::Int64(i) => i.is_none(),
            Self::Float(i) => i.is_none(),
            Self::Double(i) => i.is_none(),
            Self::Text(t) => t.is_none(),
            Self::Enum(e, _) => e.is_none(),
            Self::EnumArray(e, _) => e.is_none(),
            Self::Bytes(b) => b.is_none(),
            Self::Boolean(b) => b.is_none(),
            Self::Char(c) => c.is_none(),
            Self::Array(v) => v.is_none(),
            Self::Xml(s) => s.is_none(),
            Self::Numeric(r) => r.is_none(),
            Self::Uuid(u) => u.is_none(),
            Self::DateTime(dt) => dt.is_none(),
            Self::Date(d) => d.is_none(),
            Self::Time(t) => t.is_none(),
            Self::Json(json) => json.is_none(),
        }
    }

    /// `true` if the `Value` is text.
    pub(crate) fn is_text(&self) -> bool {
        matches!(self, Self::Text(_))
    }

    /// Returns a &str if the value is text, otherwise `None`.
    pub(crate) fn as_str(&self) -> Option<&str> {
        match self {
            Self::Text(Some(cow)) => Some(cow.borrow()),
            Self::Bytes(Some(cow)) => std::str::from_utf8(cow.as_ref()).ok(),
            _ => None,
        }
    }

    /// Returns a char if the value is a char, otherwise `None`.
    pub(crate) fn as_char(&self) -> Option<char> {
        match self {
            Self::Char(c) => *c,
            _ => None,
        }
    }

    /// Returns a cloned String if the value is text, otherwise `None`.
    pub(crate) fn to_string(&self) -> Option<String> {
        match self {
            Self::Text(Some(cow)) => Some(cow.to_string()),
            Self::Bytes(Some(cow)) => std::str::from_utf8(cow.as_ref()).map(|s| s.to_owned()).ok(),
            _ => None,
        }
    }

    /// Transforms the `Value` to a `String` if it's text,
    /// otherwise `None`.
    pub(crate) fn into_string(self) -> Option<String> {
        match self {
            Self::Text(Some(cow)) => Some(cow.into_owned()),
            Self::Bytes(Some(cow)) => String::from_utf8(cow.into_owned()).ok(),
            _ => None,
        }
    }

    /// Returns whether this value is the `Bytes` variant.
    pub(crate) fn is_bytes(&self) -> bool {
        matches!(self, Self::Bytes(_))
    }

    /// Returns a bytes slice if the value is text or a byte slice, otherwise `None`.
    pub(crate) fn as_bytes(&self) -> Option<&[u8]> {
        match self {
            Self::Text(Some(cow)) => Some(cow.as_ref().as_bytes()),
            Self::Bytes(Some(cow)) => Some(cow.as_ref()),
            _ => None,
        }
    }

    /// Returns a cloned `Vec<u8>` if the value is text or a byte slice, otherwise `None`.
    pub(crate) fn to_bytes(&self) -> Option<Vec<u8>> {
        match self {
            Self::Text(Some(cow)) => Some(cow.to_string().into_bytes()),
            Self::Bytes(Some(cow)) => Some(cow.to_vec()),
            _ => None,
        }
    }

    /// `true` if the `Value` is a 32-bit signed integer.
    pub(crate) fn is_i32(&self) -> bool {
        matches!(self, Self::Int32(_))
    }

    /// `true` if the `Value` is a 64-bit signed integer.
    pub(crate) fn is_i64(&self) -> bool {
        matches!(self, Self::Int64(_))
    }

    /// `true` if the `Value` is a signed integer.
    pub fn is_integer(&self) -> bool {
        matches!(self, Self::Int32(_) | Self::Int64(_))
    }

    /// Returns an `i64` if the value is a 64-bit signed integer, otherwise `None`.
    pub(crate) fn as_i64(&self) -> Option<i64> {
        match self {
            Self::Int64(i) => *i,
            _ => None,
        }
    }

    /// Returns an `i32` if the value is a 32-bit signed integer, otherwise `None`.
    pub(crate) fn as_i32(&self) -> Option<i32> {
        match self {
            Self::Int32(i) => *i,
            _ => None,
        }
    }

    /// Returns an `i64` if the value is a signed integer, otherwise `None`.
    pub fn as_integer(&self) -> Option<i64> {
        match self {
            Self::Int32(i) => i.map(|i| i as i64),
            Self::Int64(i) => *i,
            _ => None,
        }
    }

    /// Returns a `f64` if the value is a double, otherwise `None`.
    pub(crate) fn as_f64(&self) -> Option<f64> {
        match self {
            Self::Double(Some(f)) => Some(*f),
            _ => None,
        }
    }

    /// Returns a `f32` if the value is a double, otherwise `None`.
    pub(crate) fn as_f32(&self) -> Option<f32> {
        match self {
            Self::Float(Some(f)) => Some(*f),
            _ => None,
        }
    }

    /// `true` if the `Value` is a numeric value or can be converted to one.

    pub(crate) fn is_numeric(&self) -> bool {
        matches!(self, Self::Numeric(_) | Self::Float(_) | Self::Double(_))
    }

    /// Returns a bigdecimal, if the value is a numeric, float or double value,
    /// otherwise `None`.

    pub(crate) fn into_numeric(self) -> Option<BigDecimal> {
        match self {
            Self::Numeric(d) => d,
            Self::Float(f) => f.and_then(BigDecimal::from_f32),
            Self::Double(f) => f.and_then(BigDecimal::from_f64),
            _ => None,
        }
    }

    /// Returns a reference to a bigdecimal, if the value is a numeric.
    /// Otherwise `None`.

    pub(crate) fn as_numeric(&self) -> Option<&BigDecimal> {
        match self {
            Self::Numeric(d) => d.as_ref(),
            _ => None,
        }
    }

    /// `true` if the `Value` is a boolean value.
    pub(crate) fn is_bool(&self) -> bool {
        match self {
            Self::Boolean(_) => true,
            // For schemas which don't tag booleans
            Self::Int32(Some(i)) if *i == 0 || *i == 1 => true,
            Self::Int64(Some(i)) if *i == 0 || *i == 1 => true,
            _ => false,
        }
    }

    /// Returns a bool if the value is a boolean, otherwise `None`.
    pub(crate) fn as_bool(&self) -> Option<bool> {
        match self {
            Self::Boolean(b) => *b,
            // For schemas which don't tag booleans
            Self::Int32(Some(i)) if *i == 0 || *i == 1 => Some(*i == 1),
            Self::Int64(Some(i)) if *i == 0 || *i == 1 => Some(*i == 1),
            _ => None,
        }
    }

    /// `true` if the `Value` is an Array.
    pub(crate) fn is_array(&self) -> bool {
        matches!(self, Self::Array(_))
    }

    /// `true` if the `Value` is of UUID type.
    pub(crate) fn is_uuid(&self) -> bool {
        matches!(self, Self::Uuid(_))
    }

    /// Returns an UUID if the value is of UUID type, otherwise `None`.
    pub(crate) fn as_uuid(&self) -> Option<Uuid> {
        match self {
            Self::Uuid(u) => *u,
            _ => None,
        }
    }

    /// `true` if the `Value` is a DateTime.
    pub(crate) fn is_datetime(&self) -> bool {
        matches!(self, Self::DateTime(_))
    }

    /// Returns a `DateTime` if the value is a `DateTime`, otherwise `None`.
    pub(crate) fn as_datetime(&self) -> Option<DateTime<Utc>> {
        match self {
            Self::DateTime(dt) => *dt,
            _ => None,
        }
    }

    /// `true` if the `Value` is a Date.
    pub(crate) fn is_date(&self) -> bool {
        matches!(self, Self::Date(_))
    }

    /// Returns a `NaiveDate` if the value is a `Date`, otherwise `None`.
    pub(crate) fn as_date(&self) -> Option<NaiveDate> {
        match self {
            Self::Date(dt) => *dt,
            _ => None,
        }
    }

    /// `true` if the `Value` is a `Time`.
    pub(crate) fn is_time(&self) -> bool {
        matches!(self, Self::Time(_))
    }

    /// Returns a `NaiveTime` if the value is a `Time`, otherwise `None`.
    pub(crate) fn as_time(&self) -> Option<NaiveTime> {
        match self {
            Self::Time(time) => *time,
            _ => None,
        }
    }

    /// `true` if the `Value` is a JSON value.
    pub(crate) fn is_json(&self) -> bool {
        matches!(self, Self::Json(_))
    }

    /// Returns a reference to a JSON Value if of Json type, otherwise `None`.
    pub(crate) fn as_json(&self) -> Option<&serde_json::Value> {
        match self {
            Self::Json(Some(j)) => Some(j),
            _ => None,
        }
    }

    /// Transforms to a JSON Value if of Json type, otherwise `None`.
    pub(crate) fn into_json(self) -> Option<serde_json::Value> {
        match self {
            Self::Json(Some(j)) => Some(j),
            _ => None,
        }
    }

    /// Returns a `Vec<T>` if the value is an array of `T`, otherwise `None`.
    pub(crate) fn into_vec<T>(self) -> Option<Vec<T>>
    where
        // Implement From<Value>
        T: TryFrom<Value<'a>>,
    {
        match self {
            Self::Array(Some(vec)) => {
                let rslt: Result<Vec<_>, _> = vec.into_iter().map(T::try_from).collect();
                match rslt {
                    Err(_) => None,
                    Ok(values) => Some(values),
                }
            }
            _ => None,
        }
    }

    /// Returns a cloned Vec<T> if the value is an array of T, otherwise `None`.
    pub(crate) fn to_vec<T>(&self) -> Option<Vec<T>>
    where
        T: TryFrom<Value<'a>>,
    {
        match self {
            Self::Array(Some(vec)) => {
                let rslt: Result<Vec<_>, _> = vec.clone().into_iter().map(T::try_from).collect();
                match rslt {
                    Err(_) => None,
                    Ok(values) => Some(values),
                }
            }
            _ => None,
        }
    }
}

value!(val: i64, Int64, val);
value!(val: i32, Int32, val);
value!(val: bool, Boolean, val);
value!(val: &'a str, Text, val.into());
value!(val: &'a String, Text, val.into());
value!(val: &'a &str, Text, (*val).into());
value!(val: String, Text, val.into());
value!(val: usize, Int64, i64::try_from(val).unwrap());
value!(val: &'a [u8], Bytes, val.into());
value!(val: f64, Double, val);
value!(val: f32, Float, val);
value!(val: DateTime<Utc>, DateTime, val);
value!(val: chrono::NaiveTime, Time, val);
value!(val: chrono::NaiveDate, Date, val);
value!(val: BigDecimal, Numeric, val);
value!(val: JsonValue, Json, val);
value!(val: Uuid, Uuid, val);

impl<'a> TryFrom<Value<'a>> for i64 {
    type Error = Error;

    fn try_from(value: Value<'a>) -> Result<i64, Self::Error> {
        value
            .typed
            .as_i64()
            .ok_or_else(|| Error::builder(ErrorKind::conversion("Not an i64")).build())
    }
}

impl<'a> TryFrom<Value<'a>> for i32 {
    type Error = Error;

    fn try_from(value: Value<'a>) -> Result<i32, Self::Error> {
        value
            .as_i32()
            .ok_or_else(|| Error::builder(ErrorKind::conversion("Not an i32")).build())
    }
}

impl<'a> TryFrom<Value<'a>> for BigDecimal {
    type Error = Error;

    fn try_from(value: Value<'a>) -> Result<BigDecimal, Self::Error> {
        value
            .into_numeric()
            .ok_or_else(|| Error::builder(ErrorKind::conversion("Not a decimal")).build())
    }
}

impl<'a> TryFrom<Value<'a>> for f64 {
    type Error = Error;

    fn try_from(value: Value<'a>) -> Result<f64, Self::Error> {
        value
            .as_f64()
            .ok_or_else(|| Error::builder(ErrorKind::conversion("Not a f64")).build())
    }
}

impl<'a> TryFrom<Value<'a>> for String {
    type Error = Error;

    fn try_from(value: Value<'a>) -> Result<String, Self::Error> {
        value
            .into_string()
            .ok_or_else(|| Error::builder(ErrorKind::conversion("Not a string")).build())
    }
}

impl<'a> TryFrom<Value<'a>> for bool {
    type Error = Error;

    fn try_from(value: Value<'a>) -> Result<bool, Self::Error> {
        value
            .as_bool()
            .ok_or_else(|| Error::builder(ErrorKind::conversion("Not a bool")).build())
    }
}

impl<'a> TryFrom<Value<'a>> for DateTime<Utc> {
    type Error = Error;

    fn try_from(value: Value<'a>) -> Result<DateTime<Utc>, Self::Error> {
        value
            .as_datetime()
            .ok_or_else(|| Error::builder(ErrorKind::conversion("Not a datetime")).build())
    }
}

impl<'a> TryFrom<&Value<'a>> for Option<std::net::IpAddr> {
    type Error = Error;

    fn try_from(value: &Value<'a>) -> Result<Option<std::net::IpAddr>, Self::Error> {
        match &value.typed {
            ValueType::Text(Some(_)) => {
                let text = value.typed.as_str().unwrap();

                match std::net::IpAddr::from_str(text) {
                    Ok(ip) => Ok(Some(ip)),
                    Err(e) => Err(e.into()),
                }
            }
            ValueType::Bytes(Some(_)) => {
                let text = value.typed.as_str().unwrap();

                match std::net::IpAddr::from_str(text) {
                    Ok(ip) => Ok(Some(ip)),
                    Err(e) => Err(e.into()),
                }
            }
            _ if value.typed.is_null() => Ok(None),
            v => {
                let kind =
                    ErrorKind::conversion(format!("Couldn't convert value of type `{v:?}` to std::net::IpAddr."));

                Err(Error::builder(kind).build())
            }
        }
    }
}

impl<'a> TryFrom<&Value<'a>> for Option<uuid::Uuid> {
    type Error = Error;

    fn try_from(value: &Value<'a>) -> Result<Option<uuid::Uuid>, Self::Error> {
        match &value.typed {
            ValueType::Uuid(uuid) => Ok(*uuid),
            ValueType::Text(Some(_)) => {
                let text = value.typed.as_str().unwrap();

                match uuid::Uuid::from_str(text) {
                    Ok(ip) => Ok(Some(ip)),
                    Err(e) => Err(e.into()),
                }
            }
            ValueType::Bytes(Some(_)) => {
                let text = value.typed.as_str().unwrap();

                match uuid::Uuid::from_str(text) {
                    Ok(ip) => Ok(Some(ip)),
                    Err(e) => Err(e.into()),
                }
            }
            _ if value.typed.is_null() => Ok(None),
            v => {
                let kind = ErrorKind::conversion(format!("Couldn't convert value of type `{v:?}` to uuid::Uuid."));

                Err(Error::builder(kind).build())
            }
        }
    }
}

/// An in-memory temporary table. Can be used in some of the databases in a
/// place of an actual table. Doesn't work in MySQL 5.7.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Values<'a> {
    pub(crate) rows: Vec<Row<'a>>,
}

impl<'a> Values<'a> {
    /// Create a new empty in-memory set of values.
    pub fn empty() -> Self {
        Self { rows: Vec::new() }
    }

    /// Create a new in-memory set of values.
    pub fn new(rows: Vec<Row<'a>>) -> Self {
        Self { rows }
    }

    /// Create a new in-memory set of values with an allocated capacity.
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            rows: Vec::with_capacity(capacity),
        }
    }

    /// Add value to the temporary table.
    pub fn push<T>(&mut self, row: T)
    where
        T: Into<Row<'a>>,
    {
        self.rows.push(row.into());
    }

    /// The number of rows in the in-memory table.
    pub fn len(&self) -> usize {
        self.rows.len()
    }

    /// True if has no rows.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn row_len(&self) -> usize {
        match self.rows.split_first() {
            Some((row, _)) => row.len(),
            None => 0,
        }
    }

    pub fn flatten_row(self) -> Option<Row<'a>> {
        let mut result = Row::with_capacity(self.len());

        for mut row in self.rows.into_iter() {
            match row.pop() {
                Some(value) => result.push(value),
                None => return None,
            }
        }

        Some(result)
    }
}

impl<'a, I, R> From<I> for Values<'a>
where
    I: Iterator<Item = R>,
    R: Into<Row<'a>>,
{
    fn from(rows: I) -> Self {
        Self {
            rows: rows.map(|r| r.into()).collect(),
        }
    }
}

impl<'a> IntoIterator for Values<'a> {
    type Item = Row<'a>;
    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.rows.into_iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn a_parameterized_value_of_ints32_can_be_converted_into_a_vec() {
        let pv = Value::array(vec![1]);
        let values: Vec<i32> = pv.typed.into_vec().expect("convert into Vec<i32>");
        assert_eq!(values, vec![1]);
    }

    #[test]
    fn a_parameterized_value_of_ints64_can_be_converted_into_a_vec() {
        let pv = Value::array(vec![1_i64]);
        let values: Vec<i64> = pv.typed.into_vec().expect("convert into Vec<i64>");
        assert_eq!(values, vec![1]);
    }

    #[test]
    fn a_parameterized_value_of_reals_can_be_converted_into_a_vec() {
        let pv = Value::array(vec![1.0]);
        let values: Vec<f64> = pv.typed.into_vec().expect("convert into Vec<f64>");
        assert_eq!(values, vec![1.0]);
    }

    #[test]
    fn a_parameterized_value_of_texts_can_be_converted_into_a_vec() {
        let pv = Value::array(vec!["test"]);
        let values: Vec<String> = pv.typed.into_vec().expect("convert into Vec<String>");
        assert_eq!(values, vec!["test"]);
    }

    #[test]
    fn a_parameterized_value_of_booleans_can_be_converted_into_a_vec() {
        let pv = Value::array(vec![true]);
        let values: Vec<bool> = pv.typed.into_vec().expect("convert into Vec<bool>");
        assert_eq!(values, vec![true]);
    }

    #[test]
    fn a_parameterized_value_of_datetimes_can_be_converted_into_a_vec() {
        let datetime = DateTime::from_str("2019-07-27T05:30:30Z").expect("parsing date/time");
        let pv = Value::array(vec![datetime]);
        let values: Vec<DateTime<Utc>> = pv.typed.into_vec().expect("convert into Vec<DateTime>");
        assert_eq!(values, vec![datetime]);
    }

    #[test]
    fn a_parameterized_value_of_an_array_cant_be_converted_into_a_vec_of_the_wrong_type() {
        let pv = Value::array(vec![1]);
        let rslt: Option<Vec<f64>> = pv.typed.into_vec();
        assert!(rslt.is_none());
    }

    #[test]
    fn display_format_for_datetime() {
        let dt: DateTime<Utc> = DateTime::from_str("2019-07-27T05:30:30Z").expect("failed while parsing date");
        let pv = Value::datetime(dt);

        assert_eq!(format!("{pv}"), "\"2019-07-27 05:30:30 UTC\"");
    }

    #[test]
    fn display_format_for_date() {
        let date = NaiveDate::from_ymd_opt(2022, 8, 11).unwrap();
        let pv = Value::date(date);

        assert_eq!(format!("{pv}"), "\"2022-08-11\"");
    }

    #[test]
    fn display_format_for_time() {
        let time = NaiveTime::from_hms_opt(16, 17, 00).unwrap();
        let pv = Value::time(time);

        assert_eq!(format!("{pv}"), "\"16:17:00\"");
    }

    #[test]
    fn display_format_for_uuid() {
        let id = Uuid::from_str("67e5504410b1426f9247bb680e5fe0c8").unwrap();
        let pv = Value::uuid(id);

        assert_eq!(format!("{pv}"), "\"67e55044-10b1-426f-9247-bb680e5fe0c8\"");
    }
}
