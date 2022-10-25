use core::fmt;

use crate::value::{Constant, Text};

/// A type of a field in the datamodel.
#[derive(Debug)]
pub enum FieldType<'a> {
    /// The field is required, rendered with only the name of the
    /// type. For example: `Int`.
    Required(Constant<&'a str>),
    /// The field is optional, rendered with a question mark after the
    /// type name. For example: `Int?`.
    Optional(Constant<&'a str>),
    /// The field is an array, rendered with square brackets after the
    /// type name. For example: `Int[]`.
    Array(Constant<&'a str>),
    /// The field is not supported by Prisma, rendered as
    /// `Unsupported(ts_vector)`.
    Unsupported(Text<&'a str>),
}

impl<'a> fmt::Display for FieldType<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Required(ref t) => t.fmt(f),
            Self::Optional(ref t) => {
                t.fmt(f)?;
                f.write_str("?")
            }
            Self::Array(ref t) => {
                t.fmt(f)?;
                f.write_str("[]")
            }
            Self::Unsupported(ref t) => {
                f.write_str("Unsupported(")?;
                t.fmt(f)?;
                f.write_str(")")
            }
        }
    }
}
