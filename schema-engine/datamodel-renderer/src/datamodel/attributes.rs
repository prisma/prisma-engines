use std::{borrow::Cow, fmt};

use crate::value::{Function, FunctionParam};

/// Defines a field attribute, wrapping a function.
///
/// ```ignore
/// model X {
///   field Int @map("lol")
///             ^^^^^^^^^^^ this
/// }
/// ```
#[derive(Debug)]
pub(super) struct FieldAttribute<'a> {
    attribute: Function<'a>,
    prefix: Option<Cow<'a, str>>,
}

impl<'a> FieldAttribute<'a> {
    pub(super) fn new(attribute: Function<'a>) -> Self {
        Self {
            attribute,
            prefix: None,
        }
    }

    /// Adds a prefix to the field attribute. Useful for native types,
    /// e.g. `attr.prefix("db")` for a type attribute renders as
    /// `@db.Type`.
    pub(super) fn prefix(&mut self, prefix: impl Into<Cow<'a, str>>) {
        self.prefix = Some(prefix.into());
    }

    /// Add a new parameter to the attribute function.
    pub fn push_param(&mut self, param: impl Into<FunctionParam<'a>>) {
        self.attribute.push_param(param.into());
    }
}

impl fmt::Display for FieldAttribute<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("@")?;

        if let Some(prefix) = &self.prefix {
            f.write_str(prefix)?;
            f.write_str(".")?;
        }

        self.attribute.fmt(f)?;

        Ok(())
    }
}

/// Defines a block attribute, wrapping a function.
///
/// ```ignore
/// model X {
///   @@map("lol")
///   ^^^^^^^^^^^^ this
/// }
/// ```
#[derive(Debug)]
pub(super) struct BlockAttribute<'a>(pub(super) Function<'a>);

impl<'a> BlockAttribute<'a> {
    /// Add a new parameter to the attribute function.
    pub fn push_param(&mut self, param: impl Into<FunctionParam<'a>>) {
        self.0.push_param(param.into());
    }
}

impl fmt::Display for BlockAttribute<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("@@")?;
        self.0.fmt(f)?;

        Ok(())
    }
}
