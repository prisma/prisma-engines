use std::{borrow::Cow, fmt};

use super::IndexOps;
use crate::{
    datamodel::attributes::FieldAttribute,
    value::{Constant, Function, Text},
};

/// Input parameters for a field in a model index definition.
#[derive(Debug, Clone)]
pub struct IndexFieldInput<'a> {
    pub(crate) name: Cow<'a, str>,
    pub(crate) sort_order: Option<Cow<'a, str>>,
    pub(crate) length: Option<u32>,
    pub(crate) ops: Option<IndexOps<'a>>,
}

impl<'a> IndexFieldInput<'a> {
    /// Create a new indexed field.
    ///
    /// ```ignore
    /// @@index([foobar])
    /// //       ^^^^^^ name
    /// ```
    pub fn new(name: impl Into<Cow<'a, str>>) -> Self {
        Self {
            name: name.into(),
            sort_order: None,
            length: None,
            ops: None,
        }
    }

    /// Define the sort order of the indexed field.
    ///
    /// ```ignore
    /// @@index([foobar(sort: Desc)])
    /// //                    ^^^^ here
    /// ```
    pub fn sort_order(&mut self, sort_order: impl Into<Cow<'a, str>>) {
        self.sort_order = Some(sort_order.into());
    }

    /// Define the length of the indexed field.
    ///
    /// ```ignore
    /// @@index([foobar(length: 32)])
    /// //                      ^^ here
    /// ```
    pub fn length(&mut self, length: u32) {
        self.length = Some(length);
    }

    /// Define index operators for the field.
    ///
    /// ```ignore
    /// @@index([foobar(ops: MinMaxFoobarOps), type: Brin])
    /// //                      ^^ here
    /// ```
    pub fn ops(&mut self, ops: IndexOps<'a>) {
        self.ops = Some(ops);
    }
}

impl<'a> From<IndexFieldInput<'a>> for Function<'a> {
    fn from(definition: IndexFieldInput<'a>) -> Self {
        let name: Vec<_> = definition
            .name
            .split('.')
            .map(|name| Constant::new_no_validate(Cow::Borrowed(name)))
            .map(|constant| constant.into_inner())
            .collect();

        let mut fun = Function::from(Constant::new_no_validate(Cow::Owned(name.join("."))));

        if let Some(length) = definition.length {
            fun.push_param(("length", Constant::new_no_validate(length)));
        }

        if let Some(sort_order) = definition.sort_order {
            fun.push_param(("sort", Constant::new_no_validate(sort_order)));
        }

        if let Some(ops) = definition.ops {
            fun.push_param(("ops", ops));
        }

        fun
    }
}

/// Options for a field-level unique attribute.
#[derive(Debug)]
pub struct UniqueFieldAttribute<'a>(FieldAttribute<'a>);

impl Default for UniqueFieldAttribute<'_> {
    fn default() -> Self {
        Self(FieldAttribute::new(Function::new("unique")))
    }
}

impl<'a> UniqueFieldAttribute<'a> {
    /// Define the sort order of the inline field index.
    ///
    /// ```ignore
    /// @unique(sort: Asc)
    /// //            ^^^ here
    /// ```
    pub fn sort_order(&mut self, value: impl Into<Cow<'a, str>>) {
        self.0.push_param(("sort", Constant::new_no_validate(value.into())));
    }

    /// Define the length of the inline field index.
    ///
    /// ```ignore
    /// @unique(length: 32)
    /// //              ^^ here
    /// ```
    pub fn length(&mut self, length: u32) {
        self.0.push_param(("length", Constant::new_no_validate(length)));
    }

    /// Define the length clustering of the inline field index.
    ///
    /// ```ignore
    /// @unique(clustered: true)
    /// //                 ^^^^ here
    /// ```
    pub fn clustered(&mut self, value: bool) {
        self.0.push_param(("clustered", Constant::new_no_validate(value)))
    }

    /// Define the constraint name.
    ///
    /// ```ignore
    /// @unique(map: "key_foo")
    /// //            ^^^^^^^ here
    /// ```
    pub fn map(&mut self, value: impl Into<Cow<'a, str>>) {
        self.0.push_param(("map", Text::new(value.into())))
    }
}

impl fmt::Display for UniqueFieldAttribute<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}
