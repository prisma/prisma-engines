#![deny(missing_docs)]

use super::{Attribute, Comment, Identifier, Span, WithAttributes, WithDocumentation, WithIdentifier, WithSpan};

/// One field in a model.
#[derive(Debug, Clone, PartialEq)]
pub struct Field {
    /// The field's type.
    pub field_type: Identifier,
    /// The name of the field.
    pub name: Identifier,
    /// The aritiy of the field.
    pub arity: FieldArity,
    /// The attributes of this field.
    pub attributes: Vec<Attribute>,
    /// The comments for this field.
    pub documentation: Option<Comment>,
    /// The location of this field in the text representation.
    pub span: Span,
    /// The location of this field in the text representation.
    pub is_commented_out: bool,
}

impl Field {
    /// The value of the `@map` attribute.
    pub(crate) fn database_name(&self) -> Option<(&str, Span)> {
        self.attributes
            .iter()
            .find(|attr| attr.name.name == "map")
            .and_then(|attr| attr.arguments.get(0))
            .and_then(|args| args.value.as_string_value())
    }
}

impl WithIdentifier for Field {
    fn identifier(&self) -> &Identifier {
        &self.name
    }
}

impl WithSpan for Field {
    fn span(&self) -> &Span {
        &self.span
    }
}

impl WithAttributes for Field {
    fn attributes(&self) -> &Vec<Attribute> {
        &self.attributes
    }
}

impl WithDocumentation for Field {
    fn documentation(&self) -> &Option<Comment> {
        &self.documentation
    }

    fn is_commented_out(&self) -> bool {
        self.is_commented_out
    }
}

/// Whether a field is required, optional (?), or list ([]).
#[derive(Copy, Debug, Clone, PartialEq)]
pub enum FieldArity {
    /// Default.
    Required,
    /// ?
    Optional,
    /// []
    List,
}
