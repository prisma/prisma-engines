use super::scalars::ScalarType;
use native_types::MsSqlType;

/// represents an available native type
#[derive(serde::Serialize, Debug)]
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

impl From<MsSqlType> for NativeTypeConstructor {
    fn from(r#type: MsSqlType) -> Self {
        let matching_types = match r#type {
            MsSqlType::TinyInt => vec![ScalarType::Int],
            MsSqlType::SmallInt => vec![ScalarType::Int],
            MsSqlType::Int => vec![ScalarType::Int],
            MsSqlType::BigInt => vec![ScalarType::Int],
            MsSqlType::Decimal(_) => vec![ScalarType::Decimal],
            MsSqlType::Numeric(_) => vec![ScalarType::Decimal],
            MsSqlType::Money => vec![ScalarType::Decimal],
            MsSqlType::SmallMoney => vec![ScalarType::Decimal],
            MsSqlType::Bit => vec![ScalarType::Boolean, ScalarType::Int],
            MsSqlType::Float(_) => vec![ScalarType::Float],
            MsSqlType::Real => vec![ScalarType::Float],
            MsSqlType::Date => vec![ScalarType::DateTime],
            MsSqlType::Time => vec![ScalarType::DateTime],
            MsSqlType::DateTime => vec![ScalarType::DateTime],
            MsSqlType::DateTime2 => vec![ScalarType::DateTime],
            MsSqlType::DateTimeOffset => vec![ScalarType::DateTime],
            MsSqlType::SmallDateTime => vec![ScalarType::DateTime],
            MsSqlType::Char(_) => vec![ScalarType::String],
            MsSqlType::NChar(_) => vec![ScalarType::String],
            MsSqlType::VarChar(_) => vec![ScalarType::String],
            MsSqlType::Text => vec![ScalarType::String],
            MsSqlType::NVarChar(_) => vec![ScalarType::String],
            MsSqlType::NText => vec![ScalarType::String],
            MsSqlType::Binary(_) => vec![ScalarType::Bytes],
            MsSqlType::VarBinary(_) => vec![ScalarType::Bytes],
            MsSqlType::Image => vec![ScalarType::Bytes],
            MsSqlType::Xml => vec![ScalarType::String],
        };

        match r#type.maximum_parameters() {
            0 => Self::without_args(r#type.kind(), matching_types),
            n => Self::with_optional_args(r#type.kind(), n, matching_types),
        }
    }
}
