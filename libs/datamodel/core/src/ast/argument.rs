use super::*;
use std::fmt::{Display, Formatter};

/// An argument, either for attributes, or for keys in source blocks.
#[derive(Debug, Clone, PartialEq)]
pub struct Argument {
    /// Name of the argument.
    pub name: Identifier,
    /// Argument value.
    pub value: Expression,
    /// Location of the argument in the text representation.
    pub span: Span,
}

impl WithIdentifier for Argument {
    fn identifier(&self) -> &Identifier {
        &self.name
    }
}

impl Display for Argument {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.name.name, self.value)
    }
}

impl Argument {
    pub fn new_string(name: &str, value: String) -> Argument {
        Argument {
            name: Identifier::new(name),
            value: Expression::StringValue(value, Span::empty()),
            span: Span::empty(),
        }
    }

    pub fn new_numeric(name: &str, value: u32) -> Argument {
        Argument {
            name: Identifier::new(name),
            value: Expression::NumericValue(value.to_string(), Span::empty()),
            span: Span::empty(),
        }
    }

    pub fn new_constant(name: &str, value: &str) -> Argument {
        Argument {
            name: Identifier::new(name),
            value: Expression::ConstantValue(String::from(value), Span::empty()),
            span: Span::empty(),
        }
    }

    pub fn new_array(name: &str, value: Vec<Expression>) -> Argument {
        Argument {
            name: Identifier::new(name),
            value: Expression::Array(value, Span::empty()),
            span: Span::empty(),
        }
    }

    pub fn new_function(name: &str, fn_name: &str, value: Vec<Expression>) -> Argument {
        Argument {
            name: Identifier::new(name),
            value: Expression::Function(fn_name.to_string(), value, Span::empty()),
            span: Span::empty(),
        }
    }

    pub fn new(name: &str, value: Expression) -> Argument {
        Argument {
            name: Identifier::new(name),
            value,
            span: Span::empty(),
        }
    }

    pub fn new_unnamed(value: Expression) -> Argument {
        Argument {
            name: Identifier::new(""),
            value,
            span: Span::empty(),
        }
    }

    pub fn is_unnamed(&self) -> bool {
        self.name.name.is_empty()
    }
}
