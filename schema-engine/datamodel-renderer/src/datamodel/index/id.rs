use std::{borrow::Cow, fmt};

use crate::{
    datamodel::attributes::{BlockAttribute, FieldAttribute},
    value::{Array, Constant, Function, Text, Value},
};

use super::IndexFieldInput;

/// Defines the id attribute in a model block.
#[derive(Debug)]
pub struct IdDefinition<'a>(BlockAttribute<'a>);

impl<'a> IdDefinition<'a> {
    /// Sets the model's primary key to the given fields.
    ///
    /// ```ignore
    /// @@id([foo, bar])
    /// //   ^^^^^^^^^^ here
    /// ```
    pub fn new(fields: impl Iterator<Item = IndexFieldInput<'a>>) -> Self {
        let mut inner = Function::new("id");

        let fields: Vec<_> = fields.map(Function::from).map(Value::Function).collect();
        inner.push_param(Value::Array(Array::from(fields)));

        Self(BlockAttribute(inner))
    }

    /// Sets a client name for the id.
    ///
    /// ```ignore
    /// @@id([foo, bar], name: "Foo")
    /// //                     ^^^^^ here
    /// ```
    pub fn name(&mut self, name: impl Into<Cow<'a, str>>) {
        self.0.push_param(("name", Text::new(name.into())));
    }

    /// The primary key constraint name.
    ///
    /// ```ignore
    /// @@id([foo, bar], map: "Foo")
    /// //                    ^^^^^ here
    /// ```
    pub fn map(&mut self, map: impl Into<Cow<'a, str>>) {
        self.0.push_param(("map", Text::new(map)));
    }

    /// The constraint clustering setting.
    ///
    /// ```ignore
    /// @@id([foo, bar], clustered: false)
    /// //                          ^^^^^ here
    /// ```
    pub fn clustered(&mut self, clustered: bool) {
        self.0.push_param(("clustered", Constant::new_no_validate(clustered)));
    }
}

impl fmt::Display for IdDefinition<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

/// Defines the id attribute in a field.
#[derive(Debug)]
pub struct IdFieldDefinition<'a>(FieldAttribute<'a>);

impl<'a> IdFieldDefinition<'a> {
    /// Makes the given field to be the model's primary key.
    ///
    /// ```ignore
    /// field Int @id
    /// //        ^^^ here
    /// ```
    pub fn new() -> Self {
        Self(FieldAttribute::new(Function::new("id")))
    }

    /// The primary key constraint name.
    ///
    /// ```ignore
    /// field Int @id(map: "Foo")
    /// //                 ^^^^^ here
    /// ```
    pub fn map(&mut self, map: impl Into<Cow<'a, str>>) {
        self.0.push_param(("map", Text::new(map.into())));
    }

    /// The constraint clustering setting.
    ///
    /// ```ignore
    /// field Int @id(clustered: false)
    /// //                       ^^^^^ here
    /// ```
    pub fn clustered(&mut self, clustered: bool) {
        self.0.push_param(("clustered", Constant::new_no_validate(clustered)));
    }

    /// The constraint sort setting.
    ///
    /// ```ignore
    /// field Int @id(sort: Desc)
    /// //                  ^^^^ here
    /// ```
    pub fn sort_order(&mut self, sort: impl Into<Cow<'a, str>>) {
        self.0.push_param(("sort", Constant::new_no_validate(sort.into())));
    }

    /// The constraint length setting.
    ///
    /// ```ignore
    /// field Int @id(length: 32)
    /// //                    ^^ here
    /// ```
    pub fn length(&mut self, length: u32) {
        self.0.push_param(("length", Constant::new_no_validate(length)));
    }
}

impl Default for IdFieldDefinition<'_> {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for IdFieldDefinition<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}
