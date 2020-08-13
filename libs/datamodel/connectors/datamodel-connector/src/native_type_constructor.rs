use super::scalars::ScalarType;

/// represents an availabe native type
pub struct NativeTypeConstructor {
    /// the name that is used in the Prisma schema when declaring the native type
    pub name: String,
    /// the number of arguments that must be provided
    pub _number_of_args: usize,
    /// the number of optional arguments
    pub _number_of_optional_args: usize,
    /// the scalar type this native type is compatible with
    pub prisma_type: ScalarType,
}

impl NativeTypeConstructor {
    pub fn without_args(name: &str, prisma_type: ScalarType) -> NativeTypeConstructor {
        NativeTypeConstructor {
            name: name.to_string(),
            _number_of_args: 0,
            _number_of_optional_args: 0,
            prisma_type,
        }
    }

    pub fn with_args(name: &str, number_of_args: usize, prisma_type: ScalarType) -> NativeTypeConstructor {
        NativeTypeConstructor {
            name: name.to_string(),
            _number_of_args: number_of_args,
            _number_of_optional_args: 0,
            prisma_type,
        }
    }

    pub fn with_optional_args(
        name: &str,
        number_of_optional_args: usize,
        prisma_type: ScalarType,
    ) -> NativeTypeConstructor {
        NativeTypeConstructor {
            name: name.to_string(),
            _number_of_args: 0,
            _number_of_optional_args: number_of_optional_args,
            prisma_type,
        }
    }
}
