use super::scalars::ScalarType;

/// represents an available native type
#[derive(serde::Serialize)]
pub struct NativeTypeConstructor {
    /// The name that is used in the Prisma schema when declaring the native type
    pub name: String,

    /// The number of arguments that must be provided
    pub _number_of_args: usize,

    /// The number of optional arguments
    pub _number_of_optional_args: usize,

    /// The scalar types this native type is compatible with
    pub prisma_types: Vec<ScalarType>,

    /// The native type is referring to a List type
    pub is_list: bool,
}

impl NativeTypeConstructor {
    pub fn without_args(name: &str, prisma_types: Vec<ScalarType>) -> NativeTypeConstructor {
        NativeTypeConstructor {
            name: name.to_string(),
            _number_of_args: 0,
            _number_of_optional_args: 0,
            prisma_types,
            is_list: false,
        }
    }

    pub fn with_args(name: &str, number_of_args: usize, prisma_types: Vec<ScalarType>) -> NativeTypeConstructor {
        NativeTypeConstructor {
            name: name.to_string(),
            _number_of_args: number_of_args,
            _number_of_optional_args: 0,
            prisma_types,
            is_list: false,
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
            is_list: false,
        }
    }

    pub fn list(mut self) -> Self {
        self.is_list = true;
        self
    }
}
