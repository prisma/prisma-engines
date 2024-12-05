use std::{borrow::Cow, fmt};

use crate::value::{Array, Constant, Function, Text, Value};

use super::attributes::FieldAttribute;

/// A field default value.
#[derive(Debug)]
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
    pub fn function(mut function: Function<'a>) -> Self {
        // Our specialty in default values, empty function params lead to
        // parentheses getting rendered unlike elsewhere.
        function.render_empty_parentheses();

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
    pub fn text(value: impl Into<Cow<'a, str>>) -> Self {
        let mut inner = Function::new("default");
        inner.push_param(Value::from(Text::new(value)));

        Self::new(inner)
    }

    /// A byte array default value, base64-encoded.
    ///
    /// ```ignore
    /// model Foo {
    ///   field String @default("deadbeef")
    ///                          ^^^^^^^^ this
    /// }
    /// ```
    pub fn bytes(value: impl Into<Cow<'a, [u8]>>) -> Self {
        let mut inner = Function::new("default");
        inner.push_param(Value::from(value.into().into_owned()));

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
    pub fn constant<T>(value: T) -> Self
    where
        T: fmt::Display + 'a,
    {
        let mut inner = Function::new("default");
        inner.push_param(Value::from(Constant::new_no_validate(value)));

        Self::new(inner)
    }

    /// An array default value.
    ///
    /// ```ignore
    /// model Foo {
    ///   field String @default([1,2,3])
    ///                          ^^^^^ this
    /// }
    /// ```
    pub fn array<T>(values: Vec<T>) -> Self
    where
        T: fmt::Display + 'a,
    {
        let mut inner = Function::new("default");
        let constant = Box::new(Array::from(values));

        inner.push_param(Value::from(Constant::new_no_validate(constant)));

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
    pub fn map(&mut self, mapped_name: impl Into<Cow<'a, str>>) {
        self.0.push_param(("map", Text::new(mapped_name)));
    }

    fn new(inner: Function<'a>) -> Self {
        Self(FieldAttribute::new(inner))
    }
}

impl fmt::Display for DefaultValue<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}
