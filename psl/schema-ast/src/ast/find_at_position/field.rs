use crate::ast::{self};

use super::{WithName, WithSpan};

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
    Attribute(&'ast str, usize, Option<&'ast str>),
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
                let mut spans: Vec<(Option<&str>, ast::Span)> = attr
                    .arguments
                    .iter()
                    .map(|arg| (arg.name.as_ref().map(|n| n.name.as_str()), arg.span()))
                    .chain(
                        attr.arguments
                            .empty_arguments
                            .iter()
                            .map(|arg| (Some(arg.name.name.as_str()), arg.name.span())),
                    )
                    .collect();
                spans.sort_by_key(|(_, span)| span.start);
                let mut arg_name = None;

                for (name, _) in spans.iter().take_while(|(_, span)| span.start < position) {
                    arg_name = Some(*name);
                }

                // If the cursor is after a trailing comma, we're not in an argument.
                if let Some(span) = attr.arguments.trailing_comma {
                    if position > span.start {
                        arg_name = None;
                    }
                }

                return FieldPosition::Attribute(attr.name(), attr_idx, arg_name.flatten());
            }
        }

        FieldPosition::Field
    }
}
