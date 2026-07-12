use crate::ast::{self};

use super::{AttributePosition, WithName, WithSpan};

/// In a scalar field.
#[derive(Debug)]
pub enum FieldPosition<'ast> {
    /// Nowhere specific inside the field
    Field,
    /// In the field's name
    /// ```prisma
    /// model People {
    ///     id    String @id
    ///     field Float
    /// //  ^^^^^
    /// }
    /// ```
    Name(&'ast str),
    /// In the field's type definition
    /// ```prisma
    /// model People {
    ///     id    String @id
    ///     field Float
    /// //        ^^^^^
    /// }
    /// ```
    Type(&'ast str),
    /// In an attribute. (name, idx, optional arg)
    /// ```prisma
    /// model People {
    ///     id    String @id
    /// //              ^^^^
    ///     field Float
    /// }
    /// ```
    // Attribute(&'ast str, usize, Option<&'ast str>),
    Attribute(&'ast str, usize, AttributePosition<'ast>),
}

impl<'ast> FieldPosition<'ast> {
    pub(crate) fn new(field: &'ast ast::Field, position: usize) -> FieldPosition<'ast> {
        if field.name.span.contains(position) {
            return FieldPosition::Name(field.name());
        }

        if field.field_type.span().contains(position) {
            return FieldPosition::Type(field.field_type.name());
        }

        for (attr_idx, attr) in field.attributes.iter().enumerate() {
            if attr.span().contains(position) {
                // We can't go by Span::contains() because we also care about the empty space
                // between arguments and that's hard to capture in the pest grammar.
                return FieldPosition::Attribute(attr.name(), attr_idx, AttributePosition::new(attr, position));
            }
        }

        FieldPosition::Field
    }
}
