use serde::de::Unexpected;
use serde::{
    de::{self, Visitor},
    Deserialize, Deserializer, Serialize, Serializer,
};
use std::{convert::TryFrom, fmt, io, str::FromStr};

/// A parameter given to the SQL Server type. In many cases a number, but for
/// some variants could also be `max`, allowing the value to be taken from the
/// row to a larger heap.
#[derive(Debug, Clone, PartialEq, Copy)]
pub enum TypeParameter {
    /// Number of bytes or characters the type can use.
    Number(u16),
    /// Stores the data outside of the row, allowing maximum of two gigabytes of
    /// storage.
    Max,
}

impl TypeParameter {
    pub fn is_max(s: &str) -> bool {
        s.split(",")
            .map(|s| s.trim())
            .any(|s| matches!(s, "max" | "MAX" | "Max" | "MaX" | "maX" | "mAx"))
    }
}

impl FromStr for TypeParameter {
    type Err = io::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if TypeParameter::is_max(s) {
            Ok(TypeParameter::Max)
        } else {
            s.parse().map(TypeParameter::Number).map_err(|_| {
                let kind = io::ErrorKind::InvalidInput;
                io::Error::new(kind, "Allowed inputs: `u16` or `max`")
            })
        }
    }
}

impl Serialize for TypeParameter {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Self::Number(num) => serializer.serialize_u16(*num),
            Self::Max => serializer.serialize_str("Max"),
        }
    }
}

impl<'de> Deserialize<'de> for TypeParameter {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_any(TypeParameterVisitor)
    }
}

struct TypeParameterVisitor;

impl<'de> Visitor<'de> for TypeParameterVisitor {
    type Value = TypeParameter;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        write!(formatter, "either `max` or a number")
    }

    fn visit_u8<E>(self, value: u8) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(TypeParameter::Number(value as u16))
    }

    fn visit_u16<E>(self, value: u16) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(TypeParameter::Number(value))
    }

    fn visit_u32<E>(self, value: u32) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        u16::try_from(value)
            .map(TypeParameter::Number)
            .map_err(|_| E::invalid_value(Unexpected::Unsigned(value as u64), &self))
    }

    fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        u16::try_from(value)
            .map(TypeParameter::Number)
            .map_err(|_| E::invalid_value(Unexpected::Unsigned(value), &self))
    }

    fn visit_i8<E>(self, value: i8) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        u16::try_from(value)
            .map(TypeParameter::Number)
            .map_err(|_| E::invalid_value(Unexpected::Signed(value as i64), &self))
    }

    fn visit_i16<E>(self, value: i16) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        u16::try_from(value)
            .map(TypeParameter::Number)
            .map_err(|_| E::invalid_value(Unexpected::Signed(value as i64), &self))
    }

    fn visit_i32<E>(self, value: i32) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        u16::try_from(value)
            .map(TypeParameter::Number)
            .map_err(|_| E::invalid_value(Unexpected::Signed(value as i64), &self))
    }

    fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        u16::try_from(value)
            .map(TypeParameter::Number)
            .map_err(|_| E::invalid_value(Unexpected::Signed(value), &self))
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        value
            .parse()
            .map_err(|_| E::invalid_value(Unexpected::Str(value), &self))
    }
}

impl<T> From<T> for TypeParameter
where
    T: Into<u16>,
{
    fn from(t: T) -> Self {
        Self::Number(t.into())
    }
}

impl fmt::Display for TypeParameter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Number(num) => write!(f, "{}", num),
            Self::Max => write!(f, "max"),
        }
    }
}
