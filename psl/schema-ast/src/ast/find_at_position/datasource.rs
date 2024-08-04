use super::{PropertyPosition, WithName};
use crate::ast::{self};

#[derive(Debug)]
pub enum SourcePosition<'ast> {
    /// In the general datasource
    Source,
    /// In the datasource's name
    /// ```prisma
    /// datasource db {
    /// //         ^^
    ///     provider = "mongodb"
    ///     url      = env("DATABASE_URL")
    /// }
    /// ```
    Name(&'ast str),
    /// In a property
    /// ```prisma
    /// datasource db {
    ///     provider = "mongodb"
    /// //  ^^^^^^^^^^^^^^^^^^^^
    ///     url      = env("DATABASE_URL")
    /// }
    /// ```
    Property(&'ast str, PropertyPosition<'ast>),
    /// Outside of the braces
    Outer,
}

impl<'ast> SourcePosition<'ast> {
    pub(crate) fn new(source: &'ast ast::SourceConfig, position: usize) -> Self {
        if source.name.span.contains(position) {
            return SourcePosition::Name(source.name());
        }

        for property in &source.properties {
            if property.span.contains(position) {
                return SourcePosition::Property(&property.name.name, PropertyPosition::new(property, position));
            }
        }

        if source.inner_span.contains(position) {
            return SourcePosition::Source;
        }

        SourcePosition::Outer
    }
}
