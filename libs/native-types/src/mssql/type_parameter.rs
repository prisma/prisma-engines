use crate::ParseTypeParameter;
use crate::{NativeTypeError, NativeTypeParameter};
use serde::{Deserialize, Serialize};
use std::{fmt, str::FromStr};

/// A parameter given to the SQL Server type. In many cases a number, but for
/// some variants could also be `max`, allowing the value to be taken from the
/// row to a larger heap.
#[derive(Debug, Clone, PartialEq, Copy, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum MsSqlTypeParameter {
    /// Number of bytes or characters the type can use.
    Number(u64),
    /// Stores the data outside of the row, allowing maximum of two gigabytes of
    /// storage.
    Max,
}

impl MsSqlTypeParameter {
    pub(crate) fn is_max(s: &str) -> bool {
        s.split(",")
            .map(|s| s.trim())
            .any(|s| matches!(s, "max" | "MAX" | "Max" | "MaX" | "maX" | "mAx"))
    }
}

impl FromStr for MsSqlTypeParameter {
    type Err = NativeTypeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if Self::is_max(s) {
            Ok(Self::Max)
        } else {
            s.parse()
                .map(Self::Number)
                .map_err(|_| NativeTypeError::invalid_parameter(s, "unsigned number or `max`", super::DATABASE_NAME))
        }
    }
}

impl From<MsSqlTypeParameter> for NativeTypeParameter {
    fn from(this: MsSqlTypeParameter) -> Self {
        match this {
            MsSqlTypeParameter::Number(u) => Self::number(u),
            MsSqlTypeParameter::Max => Self::literal("Max"),
        }
    }
}

impl<T> From<T> for MsSqlTypeParameter
where
    T: Into<u64>,
{
    fn from(t: T) -> Self {
        Self::Number(t.into())
    }
}

impl fmt::Display for MsSqlTypeParameter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Number(num) => write!(f, "{}", num),
            Self::Max => write!(f, "Max"),
        }
    }
}

impl ParseTypeParameter<MsSqlTypeParameter> for &str {
    fn as_param(&self, context: &str) -> crate::Result<MsSqlTypeParameter>
    where
        Self: Sized,
    {
        self.parse()
            .map_err(|_| NativeTypeError::invalid_parameter(self, "u16", context))
    }
}
