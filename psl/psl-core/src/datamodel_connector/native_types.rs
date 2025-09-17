use diagnostics::{DatamodelError, Span};
use parser_database::{ExtensionTypeEntry, ParserDatabase, ScalarFieldType};

use std::{any::Any, borrow::Cow, fmt, sync::Arc};

/// Represents an available native type.
#[derive(Debug, Clone)]
pub struct NativeTypeConstructor {
    /// The name that is used in the Prisma schema when declaring the native type
    pub name: Cow<'static, str>,

    /// The number of arguments that must be provided
    pub number_of_args: usize,

    /// The number of optional arguments
    pub number_of_optional_args: usize,

    /// The scalar types this native type is compatible with
    pub allowed_types: Cow<'static, [AllowedType]>,
}

impl From<&ExtensionTypeEntry<'_>> for NativeTypeConstructor {
    fn from(value: &ExtensionTypeEntry<'_>) -> Self {
        NativeTypeConstructor {
            name: value.prisma_name.to_owned().into(),
            number_of_args: value.number_of_args,
            number_of_optional_args: 0,
            allowed_types: Cow::Owned(vec![AllowedType {
                field_type: ScalarFieldType::Extension(value.id),
                expected_arguments: value.db_type_modifiers.map(<[_]>::to_vec),
            }]),
        }
    }
}

#[derive(Debug, Clone)]
pub struct AllowedType {
    pub field_type: ScalarFieldType,
    pub expected_arguments: Option<Vec<String>>,
}

impl AllowedType {
    pub const fn plain(type_: ScalarFieldType) -> Self {
        Self {
            field_type: type_,
            expected_arguments: None,
        }
    }

    pub fn display<'a>(&'a self, db: &'a ParserDatabase) -> impl fmt::Display + 'a {
        DisplayAllowedType(self, db)
    }
}

pub struct DisplayAllowedType<'a>(&'a AllowedType, &'a ParserDatabase);

impl fmt::Display for DisplayAllowedType<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0.field_type.display(self.1))?;
        if let Some(expected_arguments) = &self.0.expected_arguments {
            write!(
                f,
                " (expected database type modifiers set to ({}))",
                expected_arguments.join(", ")
            )?;
        }
        Ok(())
    }
}

#[derive(Clone)]
pub struct NativeTypeInstance(Arc<dyn NativeTypeTrait>);

impl std::fmt::Debug for NativeTypeInstance {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("NativeTypeInstance(..)")
    }
}

impl NativeTypeInstance {
    pub fn new<T: Any + Send + Sync + PartialEq + 'static>(native_type: T) -> Self {
        NativeTypeInstance(Arc::new(native_type))
    }

    #[track_caller]
    pub fn downcast_ref<T: 'static + Any>(&self) -> &T {
        <dyn Any>::downcast_ref(&*self.0).unwrap()
    }
}

impl PartialEq for NativeTypeInstance {
    fn eq(&self, other: &Self) -> bool {
        self.0.dyn_eq(&*other.0)
    }
}

pub trait NativeTypeTrait: Any + Send + Sync + 'static {
    fn dyn_eq(&self, other: &dyn NativeTypeTrait) -> bool;
}

impl<A: Any + Send + Sync + PartialEq + 'static> NativeTypeTrait for A {
    fn dyn_eq(&self, other: &dyn NativeTypeTrait) -> bool {
        <dyn Any>::downcast_ref(other).is_some_and(|a| self == a)
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

#[derive(Debug)]
pub enum NativeTypeParseError<'a> {
    InvalidArgs { expected: &'static str, found: String },
    UnknownType { name: &'a str },
}

impl NativeTypeParseError<'_> {
    pub fn into_datamodel_error(self, span: Span) -> DatamodelError {
        match self {
            NativeTypeParseError::InvalidArgs { expected, found } => {
                DatamodelError::new_value_parser_error(expected, &found, span)
            }
            NativeTypeParseError::UnknownType { name } => DatamodelError::new_native_type_parser_error(name, span),
        }
    }
}
