use crate::ast::{self};

use super::{FieldPosition, WithName};

#[derive(Debug)]
pub enum CompositeTypePosition<'ast> {
    /// In the composite type, but no-where specific
    CompositeType,
    /// In the composite type's name.
    /// ```prisma
    /// type Address {
    /// //   ^^^^^^^
    ///     street String
    ///     city   String
    /// }
    /// ```
    Name(&'ast str),
    /// In a field.
    /// ```prisma
    /// type Address {
    ///     street String
    ///     city   String
    /// //  ^^^^^^^^^^^^^
    /// }
    /// ```
    Field(ast::FieldId, FieldPosition<'ast>),
}

impl<'ast> CompositeTypePosition<'ast> {
    pub(crate) fn new(composite_type: &'ast ast::CompositeType, position: usize) -> Self {
        if composite_type.name.span.contains(position) {
            return CompositeTypePosition::Name(composite_type.name());
        }

        for (field_id, field) in composite_type.iter_fields() {
            if field.span.contains(position) {
                return CompositeTypePosition::Field(field_id, FieldPosition::new(field, position));
            }
        }

        CompositeTypePosition::CompositeType
    }
}
