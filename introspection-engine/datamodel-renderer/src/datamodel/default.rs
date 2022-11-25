use std::{borrow::Cow, fmt};

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

    /// Here to cope with the initial issue of needing the DML
    /// structures. Remove when we don't generate DML in intro
    /// anymore.
    pub fn from_dml(val: &dml::DefaultValue) -> Self {
        let mut dv = match &val.kind {
            dml::DefaultKind::Single(dml::PrismaValue::String(val)) => Self::text(val.clone()),
            dml::DefaultKind::Single(dml::PrismaValue::Boolean(val)) => Self::constant(*val),
            dml::DefaultKind::Single(dml::PrismaValue::Enum(val)) => {
                Self::constant(Cow::<str>::Owned(String::clone(val)))
            }
            dml::DefaultKind::Single(dml::PrismaValue::Int(val)) => Self::constant(*val),
            dml::DefaultKind::Single(dml::PrismaValue::Uuid(val)) => Self::constant(val.as_hyphenated().to_string()),
            dml::DefaultKind::Single(dml::PrismaValue::List(ref vals)) => {
                Self::array(vals.iter().map(Value::from).collect())
            }
            dml::DefaultKind::Single(dml::PrismaValue::Json(val)) => Self::text(val.clone()),
            dml::DefaultKind::Single(dml::PrismaValue::Xml(val)) => Self::text(val.clone()),
            dml::DefaultKind::Single(dml::PrismaValue::Float(ref val)) => Self::constant(val.clone()),
            dml::DefaultKind::Single(dml::PrismaValue::BigInt(val)) => Self::constant(*val),
            dml::DefaultKind::Single(dml::PrismaValue::Bytes(val)) => Self::bytes(Cow::Owned(val.clone())),
            dml::DefaultKind::Single(dml::PrismaValue::DateTime(val)) => Self::constant(*val),
            dml::DefaultKind::Single(dml::PrismaValue::Object(_)) => unreachable!(),
            dml::DefaultKind::Single(dml::PrismaValue::Null) => unreachable!(),
            dml::DefaultKind::Expression(ref expr) => {
                let mut fun = Function::new(expr.name().to_owned());
                fun.render_empty_parentheses();

                for (arg_name, value) in expr.args() {
                    match arg_name {
                        Some(name) => fun.push_param((Cow::Owned(name.clone()), Value::from(value))),
                        None => fun.push_param(Value::from(value)),
                    }
                }

                Self::function(fun)
            }
        };

        if let Some(s) = val.db_name() {
            dv.map(s.to_owned());
        }

        dv
    }

    fn new(inner: Function<'a>) -> Self {
        Self(FieldAttribute::new(inner))
    }
}

// TODO: remove when dml is dead.
impl From<&dml::PrismaValue> for Value<'static> {
    fn from(value: &dml::PrismaValue) -> Self {
        match value {
            dml::PrismaValue::String(s) => Value::Text(Text(s.clone().into())),
            dml::PrismaValue::Boolean(v) => Value::from(Constant::new_no_validate(v)),
            dml::PrismaValue::Enum(val) => Value::from(Constant::new_no_validate(val)),
            dml::PrismaValue::Int(val) => Value::from(Constant::new_no_validate(val)),
            dml::PrismaValue::Uuid(val) => Value::from(Constant::new_no_validate(val.as_hyphenated())),
            dml::PrismaValue::List(vals) => {
                let vals = vals.iter().collect::<Vec<_>>();
                let constant = Box::new(Array::from(vals));

                Value::from(Constant::new_no_validate(constant))
            }
            dml::PrismaValue::Json(val) => Value::Text(Text(val.clone().into())),
            dml::PrismaValue::Xml(val) => Value::Text(Text(val.clone().into())),
            dml::PrismaValue::Object(_) => unreachable!(),
            dml::PrismaValue::Null => unreachable!(),
            dml::PrismaValue::DateTime(val) => Value::from(Constant::new_no_validate(val)),
            dml::PrismaValue::Float(val) => Value::from(Constant::new_no_validate(val)),
            dml::PrismaValue::BigInt(val) => Value::from(Constant::new_no_validate(val)),
            dml::PrismaValue::Bytes(val) => Value::from(val.clone()),
        }
    }
}

impl<'a> fmt::Display for DefaultValue<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}
