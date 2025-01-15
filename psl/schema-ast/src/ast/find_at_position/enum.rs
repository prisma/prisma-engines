use diagnostics::Span;

use super::{AttributePosition, WithName, WithSpan};
use crate::ast::{self};

/// A cursor position in a context.
#[derive(Debug)]
pub enum EnumPosition<'ast> {
    /// In the enum, but not somewhere more specific.
    Enum,
    /// In the enum's name.
    /// ```prisma
    /// enum Animal {
    /// //   ^^^^^^
    ///     Dog
    ///     RedPanda
    /// }
    /// ```
    Name(&'ast str, Span),
    /// In an attribute (attr name, attr index, position).
    /// ```prisma
    /// enum Animal {
    ///     Dog
    ///     RedPanda
    ///     @@map("pet")
    /// //  ^^^^^^^
    /// }
    /// ```
    EnumAttribute(&'ast str, usize, AttributePosition<'ast>),
    /// In a value.
    /// ```prisma
    /// enum Animal {
    ///     Dog
    ///     RedPanda
    /// //  ^^^^^^^
    /// }
    /// ```
    Value(ast::EnumValueId, EnumValuePosition<'ast>),
}

impl<'ast> EnumPosition<'ast> {
    pub(crate) fn new(r#enum: &'ast ast::Enum, position: usize) -> Self {
        if r#enum.name.span.contains(position) {
            return EnumPosition::Name(r#enum.name(), r#enum.name.span);
        }

        for (enum_value_id, value) in r#enum.iter_values() {
            if value.span().contains(position) {
                return EnumPosition::Value(enum_value_id, EnumValuePosition::new(value, position));
            }
        }

        for (attr_id, attr) in r#enum.attributes.iter().enumerate() {
            if attr.span().contains(position) {
                return EnumPosition::EnumAttribute(&attr.name.name, attr_id, AttributePosition::new(attr, position));
            }
        }

        EnumPosition::Enum
    }
}

/// In an enum value.
#[derive(Debug)]
pub enum EnumValuePosition<'ast> {
    /// Nowhere specific inside the value
    Value,
    /// In the name
    /// In an attribute. (name, idx, optional arg)
    /// ```prisma
    /// enum Animal {
    ///     Dog
    ///     RedPanda @map("red_panda")
    /// //  ^^^^^^^^
    /// }
    /// ```
    Name(&'ast str, Span),
    /// In an attribute. (name, idx, optional arg)
    /// ```prisma
    /// enum Animal {
    ///     Dog
    ///     RedPanda @map("red_panda")
    /// //           ^^^^^^^^^^^^^^^^^
    /// }
    /// ```
    Attribute(&'ast str, usize, AttributePosition<'ast>),
}

impl<'ast> EnumValuePosition<'ast> {
    fn new(value: &'ast ast::EnumValue, position: usize) -> EnumValuePosition<'ast> {
        if value.name.span().contains(position) {
            return EnumValuePosition::Name(value.name(), value.span());
        }

        for (attr_idx, attr) in value.attributes.iter().enumerate() {
            if attr.span().contains(position) {
                return EnumValuePosition::Attribute(attr.name(), attr_idx, AttributePosition::new(attr, position));
            }
        }

        EnumValuePosition::Value
    }
}
