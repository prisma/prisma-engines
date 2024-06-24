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
    Name(&'ast str),
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
            return EnumPosition::Name(r#enum.name());
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
    /// In an attribute. (name, idx, optional arg)
    /// In a value.
    /// ```prisma
    /// enum Animal {
    ///     Dog
    ///     RedPanda @map("red_panda")
    /// //           ^^^^^^^^^^^^^^^^^
    /// }
    /// ```
    Attribute(&'ast str, usize, Option<&'ast str>),
}

impl<'ast> EnumValuePosition<'ast> {
    fn new(value: &'ast ast::EnumValue, position: usize) -> EnumValuePosition<'ast> {
        for (attr_idx, attr) in value.attributes.iter().enumerate() {
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

                return EnumValuePosition::Attribute(attr.name(), attr_idx, arg_name.flatten());
            }
        }

        EnumValuePosition::Value
    }
}
