use std::borrow::Cow;

use crate::value::{Constant, ConstantNameValidationError, Function};

/// Input parameters for a field in a model index definition.
#[derive(Debug, Clone, Copy)]
pub struct IndexFieldInput<'a> {
    pub(super) name: &'a str,
    pub(super) sort_order: Option<&'a str>,
    pub(super) length: Option<u32>,
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
            name,
            sort_order: None,
            length: None,
        }
    }

    /// Define the sort order of the indexed field.
    ///
    /// ```ignore
    /// @@index([foobar(sort: Desc)])
    /// //                    ^^^^ here
    /// ```
    pub fn sort_order(&mut self, sort_order: &'a str) {
        self.sort_order = Some(sort_order);
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
#[derive(Debug, Default, Clone, Copy)]
pub struct IndexFieldOptions<'a> {
    pub(super) sort_order: Option<&'a str>,
    pub(super) length: Option<u32>,
    pub(super) clustered: Option<bool>,
}

impl<'a> IndexFieldOptions<'a> {
    /// Define the sort order of the inline field index.
    ///
    /// ```ignore
    /// @unique(sort: Asc)
    /// //            ^^^ here
    /// ```
    pub fn sort_order(&mut self, sort_order: &'a str) {
        self.sort_order = Some(sort_order);
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
}

impl<'a> From<IndexFieldInput<'a>> for Function<'a> {
    fn from(definition: IndexFieldInput<'a>) -> Self {
        let name = match Constant::new(definition.name) {
            Ok(c) => c,
            Err(ConstantNameValidationError::WasSanitized { sanitized }) => sanitized,
            Err(_) => Constant::new_no_validate(Cow::Borrowed(definition.name)),
        };

        let mut fun = Function::from(name);

        if let Some(length) = definition.length {
            fun.push_param(("length", Constant::new_no_validate(length)));
        }

        if let Some(sort_order) = definition.sort_order {
            fun.push_param(("sort", Constant::new_no_validate(sort_order)));
        }

        fun
    }
}
