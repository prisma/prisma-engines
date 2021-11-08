//! This module contains the models representing the Datamodel part of a Prisma schema.
//! It contains the main data structures that the engines can build upon.

use std::{fmt, ops::Deref};

pub mod composite_type;
pub mod datamodel;
pub mod default_value;
pub mod r#enum;
pub mod field;
pub mod model;
pub mod native_type_constructor;
pub mod native_type_instance;
pub mod relation_info;
pub mod scalars;
pub mod traits;

pub enum SchemaValue<T> {
    Explicit(T),
    Implicit(T),
}

impl<T> fmt::Debug for SchemaValue<T>
where
    T: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Explicit(arg0) => f.debug_tuple("Explicit").field(arg0).finish(),
            Self::Implicit(arg0) => f.debug_tuple("Generated").field(arg0).finish(),
        }
    }
}

impl<T> fmt::Display for SchemaValue<T>
where
    T: fmt::Display,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SchemaValue::Explicit(inner) => inner.fmt(f),
            SchemaValue::Implicit(inner) => inner.fmt(f),
        }
    }
}

impl<T> PartialEq for SchemaValue<T>
where
    T: PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Explicit(l0), Self::Explicit(r0)) => l0 == r0,
            (Self::Implicit(l0), Self::Explicit(r0)) => l0 == r0,
            (Self::Implicit(l0), Self::Implicit(r0)) => l0 == r0,
            (Self::Explicit(l0), Self::Implicit(r0)) => l0 == r0,
        }
    }
}

impl<T> Clone for SchemaValue<T>
where
    T: Clone,
{
    fn clone(&self) -> Self {
        match self {
            Self::Explicit(arg0) => Self::Explicit(arg0.clone()),
            Self::Implicit(arg0) => Self::Implicit(arg0.clone()),
        }
    }
}

impl<T> SchemaValue<T> {
    pub fn is_explicit(&self) -> bool {
        matches!(self, Self::Explicit(_))
    }

    pub fn is_implcit(&self) -> bool {
        matches!(self, Self::Implicit(_))
    }

    pub fn take(self) -> T {
        match self {
            SchemaValue::Explicit(inner) => inner,
            SchemaValue::Implicit(inner) => inner,
        }
    }
}

impl<T> Copy for SchemaValue<T> where T: Copy {}

impl<T> Default for SchemaValue<T>
where
    T: Default,
{
    fn default() -> Self {
        Self::Implicit(T::default())
    }
}

impl<T> Deref for SchemaValue<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        match self {
            SchemaValue::Explicit(ref t) => t,
            SchemaValue::Implicit(ref t) => t,
        }
    }
}

impl<T> AsRef<T> for SchemaValue<T> {
    fn as_ref(&self) -> &T {
        match self {
            SchemaValue::Explicit(ref t) => t,
            SchemaValue::Implicit(ref t) => t,
        }
    }
}
