use crate::connector::ColumnType;
use tiberius::{Column, ColumnType as MssqlColumnType};

impl From<&Column> for ColumnType {
    fn from(value: &Column) -> Self {
        match value.column_type() {
            MssqlColumnType::Null => ColumnType::Unknown,

            MssqlColumnType::BigVarChar
            | MssqlColumnType::BigChar
            | MssqlColumnType::NVarchar
            | MssqlColumnType::NChar
            | MssqlColumnType::Text
            | MssqlColumnType::NText => ColumnType::Text,

            MssqlColumnType::Xml => ColumnType::Xml,

            MssqlColumnType::Bit | MssqlColumnType::Bitn => ColumnType::Boolean,
            MssqlColumnType::Int1 | MssqlColumnType::Int2 | MssqlColumnType::Int4 => ColumnType::Int32,
            MssqlColumnType::Int8 | MssqlColumnType::Intn => ColumnType::Int64,

            MssqlColumnType::Datetime2
            | MssqlColumnType::Datetime4
            | MssqlColumnType::Datetime
            | MssqlColumnType::Datetimen
            | MssqlColumnType::DatetimeOffsetn => ColumnType::DateTime,

            MssqlColumnType::Float4 => ColumnType::Float,
            MssqlColumnType::Float8 | MssqlColumnType::Money | MssqlColumnType::Money4 | MssqlColumnType::Floatn => {
                ColumnType::Double
            }
            MssqlColumnType::Guid => ColumnType::Uuid,
            MssqlColumnType::Decimaln | MssqlColumnType::Numericn => ColumnType::Numeric,
            MssqlColumnType::Daten => ColumnType::Date,
            MssqlColumnType::Timen => ColumnType::Time,
            MssqlColumnType::BigVarBin | MssqlColumnType::BigBinary | MssqlColumnType::Image => ColumnType::Bytes,

            MssqlColumnType::Udt | MssqlColumnType::SSVariant => {
                unreachable!("UDT and SSVariant types are not supported by Tiberius.")
            }
        }
    }
}
