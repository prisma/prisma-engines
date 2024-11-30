use std::{borrow::Cow, fmt};

use crate::{
    datamodel::attributes::FieldAttribute,
    value::{Array, Constant, Function, Text, Value},
};

/// Defines the relation argument of a model field.
#[derive(Debug)]
pub struct Relation<'a>(FieldAttribute<'a>);

impl Default for Relation<'_> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a> Relation<'a> {
    /// Initialize an empty relation attribute.
    pub fn new() -> Self {
        Self(FieldAttribute::new(Function::new("relation")))
    }

    /// Defines the relation name. The attribute will be value-only.
    ///
    /// ```ignore
    /// @relation("foo")
    /// //         ^^^ this
    /// ```
    pub fn name(&mut self, name: impl Into<Cow<'a, str>>) {
        self.0.push_param(name.into());
    }

    /// Defines the `ON DELETE` referential action.
    ///
    /// ```ignore
    /// @relation(onDelete: DoNothing)
    /// //                  ^^^^^^^^^ this
    /// ```
    pub fn on_delete(&mut self, action: impl Into<Cow<'a, str>>) {
        self.0
            .push_param(("onDelete", Constant::new_no_validate(action.into())));
    }

    /// Defines the `ON UPDATE` referential action.
    ///
    /// ```ignore
    /// @relation(onUpdate: DoNothing)
    /// //                  ^^^^^^^^^ this
    /// ```
    pub fn on_update(&mut self, action: impl Into<Cow<'a, str>>) {
        self.0
            .push_param(("onUpdate", Constant::new_no_validate(action.into())));
    }

    /// Defines the foreign key constraint name.
    ///
    /// ```ignore
    /// @relation(map: "FK_foo")
    /// //              ^^^^^^ this
    /// ```
    pub fn map(&mut self, name: impl Into<Cow<'a, str>>) {
        self.0.push_param(("map", Text::new(name)));
    }

    /// Defines the fields array.
    ///
    /// ```ignore
    /// @relation(fields: [foo, bar])
    /// //                ^^^^^^^^^^ this
    /// ```
    pub fn fields(&mut self, fields: impl Iterator<Item = Cow<'a, str>>) {
        self.push_array_parameter("fields", fields);
    }

    /// Defines the references array.
    ///
    /// ```ignore
    /// @relation(references: [foo, bar])
    /// //                    ^^^^^^^^^^ this
    /// ```
    pub fn references(&mut self, fields: impl Iterator<Item = Cow<'a, str>>) {
        self.push_array_parameter("references", fields);
    }

    fn push_array_parameter(&mut self, param_name: &'static str, data: impl Iterator<Item = Cow<'a, str>>) {
        let fields: Vec<_> = data.map(Value::Constant).collect();

        if !fields.is_empty() {
            self.0.push_param((param_name, Array::from(fields)));
        }
    }
}

impl fmt::Display for Relation<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}
