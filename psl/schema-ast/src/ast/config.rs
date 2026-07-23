use crate::ast::{Expression, Identifier, Span, WithSpan};

use super::WithIdentifier;

/// A named property in a config block.
///
/// ```ignore
/// datasource db {
///     provider = env("PROVIDER")
///     ^^^^^^^^^^^^^^^^^^^^^^^^^^
/// }
/// ```
#[derive(Debug, Clone)]
pub struct ConfigBlockProperty {
    /// The property name.
    ///
    /// ```ignore
    /// datasource db {
    ///     provider = env("PROVIDER")
    ///     ^^^^^^^^
    /// }
    /// ```
    pub(crate) name: Identifier,
    /// The property value.
    ///
    /// ```ignore
    /// datasource db {
    ///     provider = env("PROVIDER")
    ///                ^^^^^^^^^^^^^^^
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
