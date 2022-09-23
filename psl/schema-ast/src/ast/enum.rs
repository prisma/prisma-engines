use super::{Attribute, Comment, Identifier, Span, WithAttributes, WithDocumentation, WithIdentifier, WithSpan};

/// An enum declaration. Enumeration can either be in the database schema, or completely a Prisma level concept.
///
/// PostgreSQL stores enums in a schema, while in MySQL the information is in
/// the table definition. On MongoDB the enumerations are handled in the Query
/// Engine.
#[derive(Debug, Clone)]
pub struct Enum {
    /// The name of the enum.
    ///
    /// ```ignore
    /// enum Foo { ... }
    ///      ^^^
    /// ```
    pub name: Identifier,
    /// The values of the enum.
    ///
    /// ```ignore
    /// enum Foo {
    ///   Value1
    ///   ^^^^^^
    ///   Value2
    ///   ^^^^^^
    /// }
    /// ```
    pub values: Vec<EnumValue>,
    /// The attributes of this enum.
    ///
    /// ```ignore
    /// enum Foo {
    ///   Value1
    ///   Value2
    ///
    ///   @@map("1Foo")
    ///   ^^^^^^^^^^^^^
    /// }
    /// ```
    pub attributes: Vec<Attribute>,
    /// The comments for this enum.
    ///
    /// ```ignore
    /// /// Lorem ipsum
    ///     ^^^^^^^^^^^
    /// enum Foo {
    ///   Value1
    ///   Value2
    /// }
    /// ```
    pub(crate) documentation: Option<Comment>,
    /// The location of this enum in the text representation.
    pub span: Span,
}

impl WithIdentifier for Enum {
    fn identifier(&self) -> &Identifier {
        &self.name
    }
}

impl WithSpan for Enum {
    fn span(&self) -> Span {
        self.span
    }
}

impl WithAttributes for Enum {
    fn attributes(&self) -> &[Attribute] {
        &self.attributes
    }
}

impl WithDocumentation for Enum {
    fn documentation(&self) -> Option<&str> {
        self.documentation.as_ref().map(|doc| doc.text.as_str())
    }
}

/// An enum value definition.
#[derive(Debug, Clone)]
pub struct EnumValue {
    /// The name of the enum value as it will be exposed by the api.
    pub name: Identifier,
    pub attributes: Vec<Attribute>,
    pub(crate) documentation: Option<Comment>,
    /// The location of this enum value in the text representation.
    pub span: Span,
}

impl WithIdentifier for EnumValue {
    fn identifier(&self) -> &Identifier {
        &self.name
    }
}

impl WithAttributes for EnumValue {
    fn attributes(&self) -> &[Attribute] {
        &self.attributes
    }
}

impl WithSpan for EnumValue {
    fn span(&self) -> Span {
        self.span
    }
}

impl WithDocumentation for EnumValue {
    fn documentation(&self) -> Option<&str> {
        self.documentation.as_ref().map(|doc| doc.text.as_str())
    }
}
