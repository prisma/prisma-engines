use crate::datamodel_connector::ScalarType;
use std::{any::Any, sync::Arc};

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

#[derive(Clone)]
pub struct NativeTypeInstance(Arc<dyn Any + Send + Sync + 'static>);

impl std::fmt::Debug for NativeTypeInstance {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("NativeTypeInstance(..)")
    }
}

impl NativeTypeInstance {
    pub fn new<T: Any + Send + Sync + 'static>(native_type: T) -> Self {
        NativeTypeInstance(Arc::new(native_type))
    }

    #[track_caller]
    pub fn downcast_ref<T: 'static + Any>(&self) -> &T {
        self.0.downcast_ref().unwrap()
    }
}

pub trait NativeTypeArguments: Sized {
    const DESCRIPTION: &'static str;
    const OPTIONAL_ARGUMENTS_COUNT: usize;
    const REQUIRED_ARGUMENTS_COUNT: usize;
    fn from_parts(parts: &[String]) -> Option<Self>;
    fn to_parts(&self) -> Vec<String>;
}

impl<T> NativeTypeArguments for Option<T>
where
    T: NativeTypeArguments,
{
    const DESCRIPTION: &'static str = T::DESCRIPTION;
    const OPTIONAL_ARGUMENTS_COUNT: usize = T::REQUIRED_ARGUMENTS_COUNT;
    const REQUIRED_ARGUMENTS_COUNT: usize = 0;

    fn to_parts(&self) -> Vec<String> {
        match self {
            Some(t) => t.to_parts(),
            None => Vec::new(),
        }
    }

    fn from_parts(parts: &[String]) -> Option<Self> {
        if parts.is_empty() {
            Some(None)
        } else {
            T::from_parts(parts).map(Some)
        }
    }
}

impl NativeTypeArguments for u32 {
    const DESCRIPTION: &'static str = "a nonnegative integer";
    const OPTIONAL_ARGUMENTS_COUNT: usize = 0;
    const REQUIRED_ARGUMENTS_COUNT: usize = 1;

    fn to_parts(&self) -> Vec<String> {
        vec![self.to_string()]
    }

    fn from_parts(parts: &[String]) -> Option<Self> {
        match parts {
            [n] => n.parse().ok(),
            _ => None,
        }
    }
}

impl NativeTypeArguments for (u32, u32) {
    const DESCRIPTION: &'static str = "two nonnegative integers";
    const REQUIRED_ARGUMENTS_COUNT: usize = 2;
    const OPTIONAL_ARGUMENTS_COUNT: usize = 0;

    fn to_parts(&self) -> Vec<String> {
        vec![self.0.to_string(), self.1.to_string()]
    }

    fn from_parts(parts: &[String]) -> Option<Self> {
        match parts {
            [a, b] => a.parse().ok().and_then(|a| b.parse().ok().map(|b| (a, b))),
            _ => None,
        }
    }
}
