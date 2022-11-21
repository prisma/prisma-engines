use std::borrow::Cow;

use super::IndexOps;
use crate::value::{Constant, Function};

/// Input parameters for a field in a model index definition.
#[derive(Debug, Clone)]
pub struct IndexFieldInput<'a> {
    pub(super) name: Cow<'a, str>,
    pub(super) sort_order: Option<Cow<'a, str>>,
    pub(super) length: Option<u32>,
    pub(super) ops: Option<IndexOps<'a>>,
}

impl<'a> IndexFieldInput<'a> {
    /// Create a new indexed field.
    ///
    /// ```ignore
    /// @@index([foobar])
    /// //       ^^^^^^ name
    /// ```
    pub fn new(name: &'a str) -> Self {
        Self {
            name: Cow::Borrowed(name),
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
}

/// Options for a field-level index attribute.
#[derive(Debug, Default, Clone)]
pub struct IndexFieldOptions<'a> {
    pub(super) sort_order: Option<Cow<'a, str>>,
    pub(super) length: Option<u32>,
    pub(super) clustered: Option<bool>,
    pub(super) map: Option<Cow<'a, str>>,
}

impl<'a> IndexFieldOptions<'a> {
    /// Define the sort order of the inline field index.
    ///
    /// ```ignore
    /// @unique(sort: Asc)
    /// //            ^^^ here
    /// ```
    pub fn sort_order(&mut self, sort_order: impl Into<Cow<'a, str>>) {
        self.sort_order = Some(sort_order.into());
    }

    /// Define the length of the inline field index.
    ///
    /// ```ignore
    /// @unique(length: 32)
    /// //              ^^ here
    /// ```
    pub fn length(&mut self, length: u32) {
        self.length = Some(length);
    }

    /// Define the length clustering of the inline field index.
    ///
    /// ```ignore
    /// @unique(clustered: true)
    /// //                 ^^^^ here
    /// ```
    pub fn clustered(&mut self, value: bool) {
        self.clustered = Some(value);
    }

    /// Define the constraint name.
    ///
    /// ```ignore
    /// @unique(map: "key_foo")
    /// //            ^^^^^^^ here
    /// ```
    pub fn map(&mut self, value: impl Into<Cow<'a, str>>) {
        self.map = Some(value.into());
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
