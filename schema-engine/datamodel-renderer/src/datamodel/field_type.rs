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

impl<'a> FieldKind<'a> {
    fn take_type(&mut self) -> Cow<'a, str> {
        match self {
            FieldKind::Required(Constant(s)) => std::mem::take(s),
            FieldKind::Optional(Constant(s)) => std::mem::take(s),
            FieldKind::Array(Constant(s)) => std::mem::take(s),
            FieldKind::RequiredUnsupported(Text(s)) => std::mem::take(s),
            FieldKind::OptionalUnsupported(Text(s)) => std::mem::take(s),
            FieldKind::ArrayUnsupported(Text(s)) => std::mem::take(s),
        }
    }
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

    /// Convert the field type to optional.
    pub fn into_optional(&mut self) {
        let inner = match self.inner {
            ref mut s @ FieldKind::Required(_) => FieldKind::Optional(Constant::new_no_validate(s.take_type())),
            ref mut s @ FieldKind::Array(_) => FieldKind::Optional(Constant::new_no_validate(s.take_type())),
            ref mut s @ FieldKind::RequiredUnsupported(_) => FieldKind::OptionalUnsupported(Text(s.take_type())),
            ref mut s @ FieldKind::ArrayUnsupported(_) => FieldKind::OptionalUnsupported(Text(s.take_type())),

            FieldKind::Optional(_) => return,
            FieldKind::OptionalUnsupported(_) => return,
        };

        self.inner = inner;
    }

    /// Convert the field type to array.
    pub fn into_array(&mut self) {
        let inner = match self.inner {
            ref mut s @ FieldKind::Required(_) => FieldKind::Array(Constant::new_no_validate(s.take_type())),
            ref mut s @ FieldKind::Optional(_) => FieldKind::Array(Constant::new_no_validate(s.take_type())),
            ref mut s @ FieldKind::RequiredUnsupported(_) => FieldKind::ArrayUnsupported(Text(s.take_type())),
            ref mut s @ FieldKind::OptionalUnsupported(_) => FieldKind::ArrayUnsupported(Text(s.take_type())),

            FieldKind::Array(_) => return,
            FieldKind::ArrayUnsupported(_) => return,
        };

        self.inner = inner;
    }

    /// Set the field type to be unsupported by Prisma.
    pub fn into_unsupported(&mut self) {
        let inner = match self.inner {
            ref mut s @ FieldKind::Required(_) => FieldKind::RequiredUnsupported(Text(s.take_type())),
            ref mut s @ FieldKind::Optional(_) => FieldKind::OptionalUnsupported(Text(s.take_type())),
            ref mut s @ FieldKind::Array(_) => FieldKind::ArrayUnsupported(Text(s.take_type())),

            _ => return,
        };

        self.inner = inner;
    }
}

impl fmt::Display for FieldType<'_> {
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
