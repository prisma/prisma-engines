use serde::{Deserialize, Serialize};
use std::{fmt, io, str::FromStr};

/// A parameter given to the SQL Server type. In many cases a number, but for
/// some variants could also be `max`, allowing the value to be taken from the
/// row to a larger heap.
#[derive(Debug, Clone, PartialEq, Copy, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum TypeParameter {
    /// Number of bytes or characters the type can use.
    Number(u64),
    /// Stores the data outside of the row, allowing maximum of two gigabytes of
    /// storage.
    Max,
}

impl TypeParameter {
    pub(crate) fn is_max(s: &str) -> bool {
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
                io::Error::new(kind, "Allowed inputs: `u64` or `max`")
            })
        }
    }
}

impl<T> From<T> for TypeParameter
where
    T: Into<u64>,
{
    fn from(t: T) -> Self {
        Self::Number(t.into())
    }
}

impl fmt::Display for TypeParameter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Number(num) => write!(f, "{}", num),
            Self::Max => write!(f, "Max"),
        }
    }
}
