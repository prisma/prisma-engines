use super::{Expression, Identifier, Span, WithSpan};
use std::fmt::{Display, Formatter};

/// A list of arguments inside parentheses.
#[derive(Debug, Clone, Default)]
pub struct ArgumentsList {
    /// The arguments themselves.
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
}

impl ArgumentsList {
    pub(crate) fn is_empty(&self) -> bool {
        self.arguments.is_empty()
    }

    pub(crate) fn iter(&self) -> std::slice::Iter<'_, Argument> {
        self.arguments.iter()
    }
}

/// An argument, either for attributes or for function call expressions.
#[derive(Debug, Clone)]
pub struct Argument {
    /// The argument name, if applicable.
    ///
    /// ```ignore
    /// @id(map: "myIndex")
    ///     ^^^
    /// ```
    pub name: Option<Identifier>,
    /// The argument value.
    ///
    /// ```ignore
    /// @id("myIndex")
    ///     ^^^^^^^^^
    /// ```
    pub value: Expression,
    /// Location of the argument in the text representation.
    pub span: Span,
}

// impl WithIdentifier for Argument {
//     fn identifier(&self) -> &Identifier {
//         &self.name
//     }
// }

impl Display for Argument {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if let Some(name) = &self.name {
            f.write_str(&name.name)?;
            f.write_str(":")?;
        }
        Display::fmt(&self.value, f)
    }
}

impl Argument {
    pub fn new_string(name: &str, value: String) -> Argument {
        assert!(!name.is_empty());
        Argument {
            name: Some(Identifier::new(name)),
            value: Expression::StringValue(value, Span::empty()),
            span: Span::empty(),
        }
    }

    pub fn new_numeric(name: &str, value: u32) -> Argument {
        assert!(!name.is_empty());
        Argument {
            name: Some(Identifier::new(name)),
            value: Expression::NumericValue(value.to_string(), Span::empty()),
            span: Span::empty(),
        }
    }

    pub fn new_constant(name: &str, value: &str) -> Argument {
        assert!(!name.is_empty());
        Argument {
            name: Some(Identifier::new(name)),
            value: Expression::ConstantValue(String::from(value), Span::empty()),
            span: Span::empty(),
        }
    }

    pub fn new_array(name: &str, value: Vec<Expression>) -> Argument {
        assert!(!name.is_empty());
        Argument {
            name: Some(Identifier::new(name)),
            value: Expression::Array(value, Span::empty()),
            span: Span::empty(),
        }
    }

    pub fn new_function(name: &str, fn_name: &str, value: Vec<Argument>) -> Argument {
        assert!(!name.is_empty());
        Argument {
            name: Some(Identifier::new(name)),
            value: Expression::Function(
                fn_name.to_string(),
                ArgumentsList {
                    arguments: value,
                    ..Default::default()
                },
                Span::empty(),
            ),
            span: Span::empty(),
        }
    }

    pub fn new(name: &str, value: Expression) -> Argument {
        assert!(!name.is_empty());
        Argument {
            name: Some(Identifier::new(name)),
            value,
            span: Span::empty(),
        }
    }

    pub fn new_unnamed(value: Expression) -> Argument {
        Argument {
            name: None,
            value,
            span: Span::empty(),
        }
    }

    pub fn is_unnamed(&self) -> bool {
        self.name.is_none()
    }
}

impl WithSpan for Argument {
    fn span(&self) -> &Span {
        &self.span
    }
}

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
