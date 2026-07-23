use super::{AttributePosition, FieldPosition, WithName, WithSpan};

use crate::ast::{self};

/// A cursor position in a context.
#[derive(Debug)]
pub enum ModelPosition<'ast> {
    /// In the model, but not somewhere more specific.
    Model,
    /// In the name of the model.
    /// ```prisma
    /// model People {
    /// //    ^^^^^^
    ///     id       String     @id @map("_id")
    ///     SomeUser SomeUser[]
    /// }
    /// ```
    Name(&'ast str),
    /// In an attribute (attr name, attr index, position).
    /// ```prisma
    /// model People {
    ///     id       String     @id @map("_id")
    ///     SomeUser SomeUser[]
    ///
    ///     @@ignore
    /// //  ^^^^^^^^
    /// }
    /// ```
    ModelAttribute(&'ast str, usize, AttributePosition<'ast>),
    /// In a field.
    /// ```prisma
    /// model People {
    ///     id       String     @id @map("_id")
    /// // ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
    ///     SomeUser SomeUser[]
    /// }
    /// ```
    Field(ast::FieldId, FieldPosition<'ast>),
}

impl<'ast> ModelPosition<'ast> {
    pub(crate) fn new(model: &'ast ast::Model, position: usize) -> Self {
        if model.name.span.contains(position) {
            return ModelPosition::Name(model.name());
        }

        for (field_id, field) in model.iter_fields() {
            if field.span().contains(position) {
                return ModelPosition::Field(field_id, FieldPosition::new(field, position));
            }
        }

        for (attr_id, attr) in model.attributes.iter().enumerate() {
            if attr.span().contains(position) {
                return ModelPosition::ModelAttribute(&attr.name.name, attr_id, AttributePosition::new(attr, position));
            }
        }

        ModelPosition::Model
    }
}
