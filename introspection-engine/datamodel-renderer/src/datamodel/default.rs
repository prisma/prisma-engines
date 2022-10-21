use std::fmt;

use crate::value::{Constant, Function, Text, Value};

use super::attributes::FieldAttribute;

/// A field default value.
pub struct DefaultValue<'a>(FieldAttribute<'a>);

impl<'a> DefaultValue<'a> {
    /// A function default value.
    ///
    /// ```ignore
    /// model Foo {
    ///   field String @default(uuid())
    ///                         ^^^^ this
    /// }
    /// ```
    pub fn function(function: Function<'a>) -> Self {
        let mut inner = Function::new("default");
        inner.push_param(Value::from(function));

        Self::new(inner)
    }

    /// A textual default value.
    ///
    /// ```ignore
    /// model Foo {
    ///   field String @default("meow")
    ///                          ^^^^ this
    /// }
    /// ```
    pub fn text(value: &'a str) -> Self {
        let mut inner = Function::new("default");
        inner.push_param(Value::from(Text(value)));

        Self::new(inner)
    }

    /// A constant default value.
    ///
    /// ```ignore
    /// model Foo {
    ///   field String @default(666420)
    ///                         ^^^^^^ this
    /// }
    /// ```
    pub fn constant(value: &'a str) -> Self {
        let mut inner = Function::new("default");
        inner.push_param(Value::Constant(Constant::new_no_validate(value)));

        Self::new(inner)
    }

    /// Sets the default map argument.
    ///
    /// ```ignore
    /// model Foo {
    ///   field String @default("foo", map: "IDDQDIDKFA")
    ///                                      ^^^^^^^^^^ this
    /// }
    /// ```
    pub fn map(&mut self, mapped_name: &'a str) {
        self.0.push_param(("map", Text(mapped_name)));
    }

    fn new(inner: Function<'a>) -> Self {
        Self(FieldAttribute::new(inner))
    }
}

impl<'a> fmt::Display for DefaultValue<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}
