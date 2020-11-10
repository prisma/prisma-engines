use super::scalars::ScalarType;
use native_types::MsSqlKind;

/// represents an available native type
#[derive(serde::Serialize)]
pub struct NativeTypeConstructor {
    /// the name that is used in the Prisma schema when declaring the native type
    pub name: String,
    /// the number of arguments that must be provided
    pub _number_of_args: usize,
    /// the number of optional arguments
    pub _number_of_optional_args: usize,
    /// the scalar types this native type is compatible with
    pub prisma_types: Vec<ScalarType>,
}

impl NativeTypeConstructor {
    pub fn without_args(name: &str, prisma_types: Vec<ScalarType>) -> NativeTypeConstructor {
        NativeTypeConstructor {
            name: name.to_string(),
            _number_of_args: 0,
            _number_of_optional_args: 0,
            prisma_types,
        }
    }

    pub fn with_args(name: &str, number_of_args: usize, prisma_types: Vec<ScalarType>) -> NativeTypeConstructor {
        NativeTypeConstructor {
            name: name.to_string(),
            _number_of_args: number_of_args,
            _number_of_optional_args: 0,
            prisma_types,
        }
    }

    pub fn with_optional_args(
        name: &str,
        number_of_optional_args: usize,
        prisma_types: Vec<ScalarType>,
    ) -> NativeTypeConstructor {
        NativeTypeConstructor {
            name: name.to_string(),
            _number_of_args: 0,
            _number_of_optional_args: number_of_optional_args,
            prisma_types,
        }
    }
}

impl From<MsSqlKind> for NativeTypeConstructor {
    fn from(kind: MsSqlKind) -> Self {
        let matching_types = match kind {
            MsSqlKind::TinyInt => vec![ScalarType::Int],
            MsSqlKind::SmallInt => vec![ScalarType::Int],
            MsSqlKind::Int => vec![ScalarType::Int],
            MsSqlKind::BigInt => vec![ScalarType::Int],
            MsSqlKind::Decimal => vec![ScalarType::Decimal],
            MsSqlKind::Numeric => vec![ScalarType::Decimal],
            MsSqlKind::Money => vec![ScalarType::Decimal],
            MsSqlKind::SmallMoney => vec![ScalarType::Decimal],
            MsSqlKind::Bit => vec![ScalarType::Boolean, ScalarType::Int],
            MsSqlKind::Float => vec![ScalarType::Float],
            MsSqlKind::Real => vec![ScalarType::Float],
            MsSqlKind::Date => vec![ScalarType::DateTime],
            MsSqlKind::Time => vec![ScalarType::DateTime],
            MsSqlKind::DateTime => vec![ScalarType::DateTime],
            MsSqlKind::DateTime2 => vec![ScalarType::DateTime],
            MsSqlKind::DateTimeOffset => vec![ScalarType::DateTime],
            MsSqlKind::SmallDateTime => vec![ScalarType::DateTime],
            MsSqlKind::Char => vec![ScalarType::String],
            MsSqlKind::NChar => vec![ScalarType::String],
            MsSqlKind::VarChar => vec![ScalarType::String],
            MsSqlKind::Text => vec![ScalarType::String],
            MsSqlKind::NVarChar => vec![ScalarType::String],
            MsSqlKind::NText => vec![ScalarType::String],
            MsSqlKind::Binary => vec![ScalarType::Bytes],
            MsSqlKind::VarBinary => vec![ScalarType::Bytes],
            MsSqlKind::Image => vec![ScalarType::Bytes],
            MsSqlKind::Xml => vec![ScalarType::String],
        };

        match kind.maximum_parameters() {
            0 => Self::without_args(kind.as_ref(), matching_types),
            n => Self::with_optional_args(kind.as_ref(), n, matching_types),
        }
    }
}
