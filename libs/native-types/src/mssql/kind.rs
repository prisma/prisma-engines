use serde::{Deserialize, Serialize};
use std::{fmt, io, str::FromStr};

/// A name of a type in SQL Server.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum MsSqlKind {
    TinyInt,
    SmallInt,
    Int,
    BigInt,
    Decimal,
    Numeric,
    Money,
    SmallMoney,
    Bit,
    Float,
    Real,
    Date,
    Time,
    DateTime,
    DateTime2,
    DateTimeOffset,
    SmallDateTime,
    Char,
    NChar,
    VarChar,
    Text,
    NVarChar,
    NText,
    Binary,
    VarBinary,
    Image,
    Xml,
}

impl MsSqlKind {
    /// The type kind can hold either zero or the resulting number of
    /// parameters.
    pub fn maximum_parameters(&self) -> usize {
        match self {
            MsSqlKind::TinyInt => 0,
            MsSqlKind::SmallInt => 0,
            MsSqlKind::Int => 0,
            MsSqlKind::BigInt => 0,
            MsSqlKind::Decimal => 2,
            MsSqlKind::Numeric => 2,
            MsSqlKind::Money => 0,
            MsSqlKind::SmallMoney => 0,
            MsSqlKind::Bit => 0,
            MsSqlKind::Float => 1,
            MsSqlKind::Real => 0,
            MsSqlKind::Date => 0,
            MsSqlKind::Time => 0,
            MsSqlKind::DateTime => 0,
            MsSqlKind::DateTime2 => 0,
            MsSqlKind::DateTimeOffset => 0,
            MsSqlKind::SmallDateTime => 0,
            MsSqlKind::Char => 1,
            MsSqlKind::NChar => 1,
            MsSqlKind::VarChar => 1,
            MsSqlKind::Text => 0,
            MsSqlKind::NVarChar => 1,
            MsSqlKind::NText => 0,
            MsSqlKind::Binary => 1,
            MsSqlKind::VarBinary => 1,
            MsSqlKind::Image => 0,
            MsSqlKind::Xml => 0,
        }
    }

    pub fn allows_max_variant(&self) -> bool {
        matches!(self, MsSqlKind::VarChar | MsSqlKind::NVarChar | MsSqlKind::VarBinary)
    }
}

impl AsRef<str> for MsSqlKind {
    fn as_ref(&self) -> &str {
        match self {
            MsSqlKind::TinyInt => "TinyInt",
            MsSqlKind::SmallInt => "SmallInt",
            MsSqlKind::Int => "Int",
            MsSqlKind::BigInt => "BigInt",
            MsSqlKind::Decimal => "Decimal",
            MsSqlKind::Numeric => "Numeric",
            MsSqlKind::Money => "Money",
            MsSqlKind::SmallMoney => "SmallMoney",
            MsSqlKind::Bit => "Bit",
            MsSqlKind::Float => "Float",
            MsSqlKind::Real => "Real",
            MsSqlKind::Date => "Date",
            MsSqlKind::Time => "Time",
            MsSqlKind::DateTime => "DateTime",
            MsSqlKind::DateTime2 => "DateTime2",
            MsSqlKind::DateTimeOffset => "DateTimeOffset",
            MsSqlKind::SmallDateTime => "SmallDateTime",
            MsSqlKind::Char => "Char",
            MsSqlKind::NChar => "NChar",
            MsSqlKind::VarChar => "VarChar",
            MsSqlKind::Text => "Text",
            MsSqlKind::NVarChar => "NVarChar",
            MsSqlKind::NText => "NText",
            MsSqlKind::Binary => "Binary",
            MsSqlKind::VarBinary => "VarBinary",
            MsSqlKind::Image => "Image",
            MsSqlKind::Xml => "Xml",
        }
    }
}

impl FromStr for MsSqlKind {
    type Err = io::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let k = match s.to_lowercase().as_str() {
            "tinyint" => MsSqlKind::TinyInt,
            "smallint" => MsSqlKind::SmallInt,
            "int" => MsSqlKind::Int,
            "bigint" => MsSqlKind::BigInt,
            "decimal" => MsSqlKind::Decimal,
            "numeric" => MsSqlKind::Numeric,
            "money" => MsSqlKind::Money,
            "smallmoney" => MsSqlKind::SmallMoney,
            "bit" => MsSqlKind::Bit,
            "float" => MsSqlKind::Float,
            "real" => MsSqlKind::Real,
            "date" => MsSqlKind::Date,
            "time" => MsSqlKind::Time,
            "datetime" => MsSqlKind::DateTime,
            "datetime2" => MsSqlKind::DateTime2,
            "datetimeoffset" => MsSqlKind::DateTimeOffset,
            "smalldatetime" => MsSqlKind::SmallDateTime,
            "char" => MsSqlKind::Char,
            "nchar" => MsSqlKind::NChar,
            "varchar" => MsSqlKind::VarChar,
            "text" => MsSqlKind::Text,
            "nvarchar" => MsSqlKind::NVarChar,
            "ntext" => MsSqlKind::NText,
            "binary" => MsSqlKind::Binary,
            "varbinary" => MsSqlKind::VarBinary,
            "image" => MsSqlKind::Image,
            "xml" => MsSqlKind::Xml,
            k => {
                let kind = io::ErrorKind::InvalidInput;
                Err(io::Error::new(kind, format!("Invalid SQL Server type: `{}`", k)))?
            }
        };

        Ok(k)
    }
}

impl fmt::Display for MsSqlKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_ref())
    }
}
