use super::{
    Attribute, Comment, Identifier, Span, WithAttributes, WithDocumentation, WithIdentifier, WithName, WithSpan,
};

/// A field definition in a model or a composite type.
#[derive(Debug, Clone)]
pub struct Field {
    /// The field's type.
    ///
    /// ```ignore
    /// name String
    ///      ^^^^^^
    /// ```
    pub field_type: FieldType,
    /// The name of the field.
    ///
    /// ```ignore
    /// name String
    /// ^^^^
    /// ```
    pub name: Identifier,
    /// The arity of the field.
    pub arity: FieldArity,
    /// The attributes of this field.
    ///
    /// ```ignore
    /// name String @id @default("lol")
    ///             ^^^^^^^^^^^^^^^^^^^
    /// ```
    pub attributes: Vec<Attribute>,
    /// The comments for this field.
    ///
    /// ```ignore
    /// /// Lorem ipsum
    ///     ^^^^^^^^^^^
    /// name String @id @default("lol")
    /// ```
    pub documentation: Option<Comment>,
    /// The location of this field in the text representation.
    pub span: Span,
    /// The location of this field in the text representation.
    pub is_commented_out: bool,
}

impl Field {
    /// Finds the position span of the argument in the given field attribute.
    pub fn span_for_argument(&self, attribute: &str, argument: &str) -> Option<Span> {
        self.attributes
            .iter()
            .filter(|a| a.name() == attribute)
            .flat_map(|a| a.arguments.iter())
            .filter(|a| a.name.as_ref().map(|n| n.name.as_str()) == Some(argument))
            .map(|a| a.span)
            .next()
    }

    /// Finds the position span of the given attribute.
    pub fn span_for_attribute(&self, attribute: &str) -> Option<Span> {
        self.attributes
            .iter()
            .filter(|a| a.name() == attribute)
            .map(|a| a.span)
            .next()
    }

    /// The name of the field
    pub fn name(&self) -> &str {
        &self.name.name
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
    fn attributes(&self) -> &[Attribute] {
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

/// An arity of a data model field.
#[derive(Copy, Debug, Clone, PartialEq)]
pub enum FieldArity {
    /// The field either must be in an insert statement, or the field must have
    /// a default value for the insert to succeed.
    ///
    /// ```ignore
    /// name String
    /// ```
    Required,
    /// The field does not need to be in an insert statement for the write to
    /// succeed.
    ///
    /// ```ignore
    /// name String?
    /// ```
    Optional,
    /// The field can have multiple values stored in the same column.
    ///
    /// ```ignore
    /// name String[]
    /// ```
    List,
}

impl FieldArity {
    pub fn is_list(&self) -> bool {
        matches!(self, &FieldArity::List)
    }

    pub fn is_optional(&self) -> bool {
        matches!(self, &FieldArity::Optional)
    }

    pub fn is_required(&self) -> bool {
        matches!(self, &FieldArity::Required)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum FieldType {
    Supported(Identifier),
    /// Unsupported("...")
    Unsupported(String, Span),
}

impl FieldType {
    pub fn span(&self) -> Span {
        match self {
            FieldType::Supported(ident) => ident.span,
            FieldType::Unsupported(_, span) => *span,
        }
    }

    pub fn as_unsupported(&self) -> Option<(&str, &Span)> {
        match self {
            FieldType::Unsupported(name, span) => Some((name, span)),
            FieldType::Supported(_) => None,
        }
    }
}
