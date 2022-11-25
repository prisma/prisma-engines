use core::fmt;
use std::borrow::Cow;

use crate::value::{Constant, Text};

#[derive(Debug)]
enum FieldKind<'a> {
    Required(Constant<Cow<'a, str>>),
    Optional(Constant<Cow<'a, str>>),
    Array(Constant<Cow<'a, str>>),
    RequiredUnsupported(Text<Cow<'a, str>>),
    OptionalUnsupported(Text<Cow<'a, str>>),
    ArrayUnsupported(Text<Cow<'a, str>>),
}

/// A type of a field in the datamodel.
#[derive(Debug)]
pub struct FieldType<'a> {
    inner: FieldKind<'a>,
}

impl<'a> FieldType<'a> {
    /// The field is required, rendered with only the name of the
    /// type. For example: `Int`.
    ///
    /// The name will be sanitized, removing unsupported characters.
    pub fn required(name: impl Into<Cow<'a, str>>) -> Self {
        let name = Constant::new_no_validate(name.into());

        Self {
            inner: FieldKind::Required(name),
        }
    }

    /// The field is optional, rendered with a question mark after the
    /// type name. For example: `Int?`.
    ///
    /// The name will be sanitized, removing unsupported characters.
    pub fn optional(name: impl Into<Cow<'a, str>>) -> Self {
        let name = Constant::new_no_validate(name.into());
        Self {
            inner: FieldKind::Optional(name),
        }
    }

    /// The field is an array, rendered with square brackets after the
    /// type name. For example: `Int[]`.
    ///
    /// The name will be sanitized, removing unsupported characters.
    pub fn array(name: impl Into<Cow<'a, str>>) -> Self {
        let name = Constant::new_no_validate(name.into());
        Self {
            inner: FieldKind::Array(name),
        }
    }

    /// The field is required, but not supported by Prisma, rendered
    /// as `Unsupported(ts_vector)`.
    pub fn required_unsupported(name: impl Into<Cow<'a, str>>) -> Self {
        Self {
            inner: FieldKind::RequiredUnsupported(Text(name.into())),
        }
    }

    /// The field is optional, but not supported by Prisma, rendered
    /// as `Unsupported(ts_vector)?`.
    pub fn optional_unsupported(name: impl Into<Cow<'a, str>>) -> Self {
        Self {
            inner: FieldKind::OptionalUnsupported(Text(name.into())),
        }
    }

    /// The field is optional, but not supported by Prisma, rendered
    /// as `Unsupported(ts_vector)?`.
    pub fn array_unsupported(name: impl Into<Cow<'a, str>>) -> Self {
        Self {
            inner: FieldKind::ArrayUnsupported(Text(name.into())),
        }
    }
}

impl<'a> fmt::Display for FieldType<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.inner {
            FieldKind::Required(ref t) => t.fmt(f),
            FieldKind::Optional(ref t) => {
                t.fmt(f)?;
                f.write_str("?")
            }
            FieldKind::Array(ref t) => {
                t.fmt(f)?;
                f.write_str("[]")
            }
            FieldKind::RequiredUnsupported(ref t) => {
                f.write_str("Unsupported(")?;
                t.fmt(f)?;
                f.write_str(")")
            }
            FieldKind::OptionalUnsupported(ref t) => {
                f.write_str("Unsupported(")?;
                t.fmt(f)?;
                f.write_str(")?")
            }
            FieldKind::ArrayUnsupported(ref t) => {
                f.write_str("Unsupported(")?;
                t.fmt(f)?;
                f.write_str(")[]")
            }
        }
    }
}
