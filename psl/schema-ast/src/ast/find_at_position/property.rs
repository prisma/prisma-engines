use crate::ast::{self};

use super::WithName;

#[derive(Debug)]
pub enum PropertyPosition<'ast> {
    Property,
    /// In the property's name.
    /// ```prisma
    /// datasource db {
    ///     provider = "mongodb"
    /// //  ^^^^^^^^
    ///     url      = env("DATABASE_URL")
    /// }
    /// ```
    Name(&'ast str),
    /// In the property's value.
    /// ```prisma
    /// datasource db {
    ///     provider = "mongodb"
    /// //             ^^^^^^^^^
    ///     url      = env("DATABASE_URL")
    /// }
    /// ```
    Value(&'ast str),
    /// In the property's value - specifically a function.
    /// ```prisma
    /// datasource db {
    ///     provider = "mongodb"
    ///     url      = env("DATABASE_URL")
    /// //             ^^^^^^^^^^^^^^^^^^^
    /// }
    /// ```
    FunctionValue(&'ast str),
}

impl<'ast> PropertyPosition<'ast> {
    pub(crate) fn new(property: &'ast ast::ConfigBlockProperty, position: usize) -> Self {
        if property.name.span.contains(position) {
            return PropertyPosition::Name(property.name());
        }

        if let Some(val) = &property.value {
            if val.span().contains(position) && val.is_function() {
                let func = val.as_function().unwrap();

                if func.0 == "env" {
                    return PropertyPosition::FunctionValue("env");
                }
            }
        }
        if property.span.contains(position) && !property.name.span.contains(position) {
            // TODO(@druue): this should actually just return the value string, not the name of the property the value is for
            return PropertyPosition::Value(&property.name.name);
        }

        PropertyPosition::Property
    }
}
