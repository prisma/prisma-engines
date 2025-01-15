use diagnostics::Span;

use crate::ast::{self};

use super::{WithIdentifier, WithName, WithSpan};

#[derive(Debug)]
pub enum PropertyPosition<'ast> {
    /// In the property somewhere.
    /// ```prisma
    /// datasource db {
    ///     provider = "mongodb"
    /// //  ^^^^^^^^^^^^^^^^^^^
    ///     url      = env("DATABASE_URL")
    /// }
    /// ```
    Property,
    ///```ignore
    /// * (property name, property span)
    ///```
    /// In the property's name.
    /// ```prisma
    /// datasource db {
    ///     provider = "mongodb"
    /// //  ^^^^^^^^
    ///     url      = env("DATABASE_URL")
    /// }
    /// ```
    Name(&'ast str, Span),
    ///```ignore
    /// * (property name, property span)
    ///```
    ///
    /// In the property's value.
    /// ```prisma
    /// datasource db {
    ///     provider = "mongodb"
    /// //             ^^^^^^^^^
    ///     url      = env("DATABASE_URL")
    /// }
    /// ```
    Value(&'ast str, Span),
    /// ```ignore
    /// * (function name, function span)
    /// ```
    ///
    /// In the property's value - specifically a function.
    /// ```prisma
    /// datasource db {
    ///     provider = "mongodb"
    ///     url      = env("DATABASE_URL")
    /// //             ^^^^^^^^^^^^^^^^^^^
    /// }
    /// ```
    FunctionValue(&'ast str, Span),
}

impl<'ast> PropertyPosition<'ast> {
    pub(crate) fn new(property: &'ast ast::ConfigBlockProperty, position: usize) -> Self {
        if property.name.span.contains(position) {
            return PropertyPosition::Name(property.name(), property.identifier().span());
        }

        if let Some(val) = &property.value {
            if val.span().contains(position) && val.is_function() {
                let func = val.as_function().unwrap();

                if func.0 == "env" {
                    return PropertyPosition::FunctionValue("env", func.2);
                }
            }
        }

        if property.span.contains(position) && !property.name.span.contains(position) {
            // TODO(@druue): this should actually just return the value string, not the name of the property the value is for
            return PropertyPosition::Value(&property.name.name, property.span());
        }

        PropertyPosition::Property
    }
}
