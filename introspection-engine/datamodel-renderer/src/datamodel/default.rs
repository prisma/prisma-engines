use std::fmt;

use psl::dml;

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
    pub fn text(value: &'a str) -> Self {
        let mut inner = Function::new("default");
        inner.push_param(Value::from(Text(value)));

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
    pub fn bytes(value: &'a [u8]) -> Self {
        let mut inner = Function::new("default");
        inner.push_param(Value::from(value));

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
        inner.push_param(Value::Constant(Constant::new_no_validate(Box::new(value))));

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

        inner.push_param(Value::Constant(Constant::new_no_validate(constant)));

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

    /// Here to cope with the initial issue of needing the DML
    /// structures. Remove when we don't generate DML in intro
    /// anymore.
    pub fn from_dml(val: &'a dml::DefaultValue) -> Self {
        let mut dv = match &val.kind {
            dml::DefaultKind::Single(dml::PrismaValue::String(ref val)) => Self::text(val),
            dml::DefaultKind::Single(dml::PrismaValue::Boolean(val)) => Self::constant(val),
            dml::DefaultKind::Single(dml::PrismaValue::Enum(val)) => Self::constant(val.as_str()),
            dml::DefaultKind::Single(dml::PrismaValue::Int(val)) => Self::constant(val),
            dml::DefaultKind::Single(dml::PrismaValue::Uuid(val)) => Self::constant(val.as_hyphenated()),
            dml::DefaultKind::Single(dml::PrismaValue::List(ref vals)) => {
                Self::array(vals.iter().map(Value::from).collect())
            }
            dml::DefaultKind::Single(dml::PrismaValue::Json(ref val)) => Self::text(val),
            dml::DefaultKind::Single(dml::PrismaValue::Xml(ref val)) => Self::text(val),
            dml::DefaultKind::Single(dml::PrismaValue::Float(ref val)) => Self::constant(val),
            dml::DefaultKind::Single(dml::PrismaValue::BigInt(val)) => Self::constant(val),
            dml::DefaultKind::Single(dml::PrismaValue::Bytes(ref val)) => Self::bytes(val),
            dml::DefaultKind::Single(dml::PrismaValue::DateTime(val)) => Self::constant(val),
            dml::DefaultKind::Single(dml::PrismaValue::Object(_)) => unreachable!(),
            dml::DefaultKind::Single(dml::PrismaValue::Null) => unreachable!(),
            dml::DefaultKind::Expression(ref expr) => {
                let mut fun = Function::new(expr.name());
                fun.render_empty_parentheses();

                for (arg_name, value) in expr.args() {
                    match arg_name {
                        Some(name) => fun.push_param((name.as_str(), Value::from(value))),
                        None => fun.push_param(Value::from(value)),
                    }
                }

                Self::function(fun)
            }
        };

        if let Some(s) = val.db_name() {
            dv.map(s);
        }

        dv
    }

    fn new(inner: Function<'a>) -> Self {
        Self(FieldAttribute::new(inner))
    }
}

// TODO: remove when dml is dead.
impl<'a> From<&'a dml::PrismaValue> for Value<'a> {
    fn from(value: &'a dml::PrismaValue) -> Self {
        match value {
            dml::PrismaValue::String(ref s) => Value::Text(Text(s)),
            dml::PrismaValue::Boolean(v) => Value::Constant(Constant::new_no_validate(Box::new(v))),
            dml::PrismaValue::Enum(val) => Value::Constant(Constant::new_no_validate(Box::new(val.as_str()))),
            dml::PrismaValue::Int(val) => Value::Constant(Constant::new_no_validate(Box::new(val))),
            dml::PrismaValue::Uuid(val) => Value::Constant(Constant::new_no_validate(Box::new(val.as_hyphenated()))),
            dml::PrismaValue::List(vals) => {
                let vals = vals.iter().collect::<Vec<_>>();
                let constant = Box::new(Array::from(vals));

                Value::Constant(Constant::new_no_validate(constant))
            }
            dml::PrismaValue::Json(ref val) => Value::Text(Text(val)),
            dml::PrismaValue::Xml(ref val) => Value::Text(Text(val)),
            dml::PrismaValue::Object(_) => unreachable!(),
            dml::PrismaValue::Null => unreachable!(),
            dml::PrismaValue::DateTime(val) => Value::Constant(Constant::new_no_validate(Box::new(val))),
            dml::PrismaValue::Float(val) => Value::Constant(Constant::new_no_validate(Box::new(val))),
            dml::PrismaValue::BigInt(val) => Value::Constant(Constant::new_no_validate(Box::new(val))),
            dml::PrismaValue::Bytes(val) => Value::from(val.as_slice()),
        }
    }
}

impl<'a> fmt::Display for DefaultValue<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}
