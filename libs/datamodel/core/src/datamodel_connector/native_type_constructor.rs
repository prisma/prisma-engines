use crate::datamodel_connector::ScalarType;

/// Represents an available native type.
pub struct NativeTypeConstructor {
    /// The name that is used in the Prisma schema when declaring the native type
    pub name: &'static str,

    /// The number of arguments that must be provided
    pub number_of_args: usize,

    /// The number of optional arguments
    pub number_of_optional_args: usize,

    /// The scalar types this native type is compatible with
    pub prisma_types: &'static [ScalarType],
}

impl NativeTypeConstructor {
    pub const fn without_args(name: &'static str, prisma_types: &'static [ScalarType]) -> NativeTypeConstructor {
        NativeTypeConstructor {
            name,
            number_of_args: 0,
            number_of_optional_args: 0,
            prisma_types,
        }
    }

    pub const fn with_args(
        name: &'static str,
        number_of_args: usize,
        prisma_types: &'static [ScalarType],
    ) -> NativeTypeConstructor {
        NativeTypeConstructor {
            name,
            number_of_args,
            number_of_optional_args: 0,
            prisma_types,
        }
    }

    pub const fn with_optional_args(
        name: &'static str,
        number_of_optional_args: usize,
        prisma_types: &'static [ScalarType],
    ) -> NativeTypeConstructor {
        NativeTypeConstructor {
            name,
            number_of_args: 0,
            number_of_optional_args,
            prisma_types,
        }
    }
}
