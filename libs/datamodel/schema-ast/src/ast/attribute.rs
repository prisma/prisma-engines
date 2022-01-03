use super::{Argument, Identifier, Span, WithIdentifier, WithSpan};

/// An argument with a name but no value. Example:
///
/// ```ignore
/// @relation(onDelete: )
/// ```
///
/// This is of course invalid, but we parse it in order to provide better diagnostics and
/// for autocompletion.
#[derive(Debug, Clone)]
pub struct EmptyArgument {
    pub name: Identifier,
}

/// An attribute (following `@` or `@@``) on a model, model field, enum, enum value or composite
/// type field.
#[derive(Debug, Clone)]
pub struct Attribute {
    /// The name of the attribute:
    ///
    /// ```ignore
    /// @@index([a, b, c])
    ///   ^^^^^
    /// ```
    pub name: Identifier,
    /// The arguments of the attribute.
    ///
    /// ```ignore
    /// @@index([a, b, c], map: "myidix")
    ///         ^^^^^^^^^^^^^^^^^^^^^^^^
    /// ```
    pub arguments: Vec<Argument>,
    /// The arguments without a value:
    ///
    /// ```ignore
    /// @default("george", map: )
    ///                    ^^^^
    /// ```
    pub empty_arguments: Vec<EmptyArgument>,
    /// The trailing comma at the end of the arguments list.
    ///
    /// ```ignore
    /// @relation(fields: [a, b], references: [id, name], )
    ///                                                 ^
    /// ```
    pub trailing_comma: Option<Span>,
    /// The AST span of the node.
    pub span: Span,
}

impl Attribute {
    /// Create a new attribute node from a name and a list of arguments.
    pub fn new(name: &str, arguments: Vec<Argument>) -> Attribute {
        Attribute {
            name: Identifier::new(name),
            arguments,
            span: Span::empty(),
            empty_arguments: Vec::new(),
            trailing_comma: None,
        }
    }

    /// Try to find the argument and return its span.
    pub fn span_for_argument(&self, argument: &str) -> Option<Span> {
        self.arguments.iter().find(|a| a.name.name == argument).map(|a| a.span)
    }
}

impl WithIdentifier for Attribute {
    fn identifier(&self) -> &Identifier {
        &self.name
    }
}

impl WithSpan for Attribute {
    fn span(&self) -> &Span {
        &self.span
    }
}
