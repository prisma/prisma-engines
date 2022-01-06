use super::{Argument, ArgumentsList, Identifier, Span, WithIdentifier, WithSpan};

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
    pub arguments: ArgumentsList,
    /// The AST span of the node.
    pub span: Span,
}

impl Attribute {
    /// Create a new attribute node from a name and a list of arguments.
    pub fn new(name: &str, arguments: Vec<Argument>) -> Attribute {
        Attribute {
            name: Identifier::new(name),
            arguments: ArgumentsList {
                arguments,
                ..Default::default()
            },
            span: Span::empty(),
        }
    }

    /// Try to find the argument and return its span.
    pub fn span_for_argument(&self, argument: &str) -> Option<Span> {
        self.arguments
            .iter()
            .find(|a| a.name.as_ref().map(|n| n.name.as_str()) == Some(argument))
            .map(|a| a.span)
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
