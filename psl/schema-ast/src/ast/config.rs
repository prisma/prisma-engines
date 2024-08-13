use crate::ast::{Expression, Identifier, Span, WithSpan};

use super::WithIdentifier;

/// A named property in a config block.
///
/// ```ignore
/// datasource db {
///     url = env("URL")
///     ^^^^^^^^^^^^^^^^
/// }
/// ```
#[derive(Debug, Clone)]
pub struct ConfigBlockProperty {
    /// The property name.
    ///
    /// ```ignore
    /// datasource db {
    ///     url = env("URL")
    ///     ^^^
    /// }
    /// ```
    pub(crate) name: Identifier,
    /// The property value.
    ///
    /// ```ignore
    /// datasource db {
    ///     url = env("URL")
    ///           ^^^^^^^^^^
    /// }
    /// ```
    pub value: Option<Expression>,
    /// The node span.
    pub span: Span,
}

impl WithSpan for ConfigBlockProperty {
    fn span(&self) -> Span {
        self.span
    }
}

impl WithIdentifier for ConfigBlockProperty {
    fn identifier(&self) -> &Identifier {
        &self.name
    }
}
