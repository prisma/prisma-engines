use super::*;

/// An enum declaration.
#[derive(Debug, Clone, PartialEq)]
pub struct Enum {
    /// The name of the enum.
    pub name: Identifier,
    /// The values of the enum.
    pub values: Vec<EnumValue>,
    /// The attributes of this enum.
    pub attributes: Vec<Attribute>,
    /// The comments for this enum.
    pub documentation: Option<Comment>,
    /// The location of this enum in the text representation.
    pub span: Span,
}

impl WithIdentifier for Enum {
    fn identifier(&self) -> &Identifier {
        &self.name
    }
}

impl WithSpan for Enum {
    fn span(&self) -> &Span {
        &self.span
    }
}

impl WithAttributes for Enum {
    fn attributes(&self) -> &Vec<Attribute> {
        &self.attributes
    }
}

impl WithDocumentation for Enum {
    fn documentation(&self) -> &Option<Comment> {
        &self.documentation
    }

    fn is_commented_out(&self) -> bool {
        false
    }
}

/// An enum value definition.
#[derive(Debug, Clone, PartialEq)]
pub struct EnumValue {
    /// The name of the enum value as it will be exposed by the api.
    pub name: Identifier,
    /// The enum value as it will be stored in the database.
    pub attributes: Vec<Attribute>,
    /// The location of this enum value in the text representation.
    pub documentation: Option<Comment>,
    pub span: Span,
    pub commented_out: bool,
}

impl WithIdentifier for EnumValue {
    fn identifier(&self) -> &Identifier {
        &self.name
    }
}

impl WithAttributes for EnumValue {
    fn attributes(&self) -> &Vec<Attribute> {
        &self.attributes
    }
}

impl WithSpan for EnumValue {
    fn span(&self) -> &Span {
        &self.span
    }
}
