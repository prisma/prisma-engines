use super::{Attribute, Comment, Identifier, Span, WithAttributes, WithDocumentation, WithIdentifier, WithSpan};

#[derive(Debug, Clone, PartialEq)]
pub struct Field {
    /// The field's type.
    pub field_type: FieldType,
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

#[derive(Copy, Debug, Clone, PartialEq)]
pub enum FieldArity {
    Required,
    Optional,
    List,
}

#[derive(Debug, Clone, PartialEq)]
pub enum FieldType {
    Supported(Identifier),
    Unsupported(String, Span),
}

impl FieldType {
    pub(crate) fn span(&self) -> Span {
        match self {
            FieldType::Supported(ident) => ident.span,
            FieldType::Unsupported(_, span) => *span,
        }
    }

    pub(crate) fn unwrap_supported(&self) -> &Identifier {
        match self {
            FieldType::Supported(ident) => ident,
            FieldType::Unsupported(_, _) => panic!("Unsupported in unwrap_supported()"),
        }
    }
}
