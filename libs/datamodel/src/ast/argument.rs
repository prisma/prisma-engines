use super::*;

/// An argument, either for directives, or for keys in source blocks.
#[derive(Debug, Clone)]
pub struct Argument {
    /// Name of the argument.
    pub name: Identifier,
    /// Argument value.
    pub value: Value,
    /// Location of the argument in the text representation.
    pub span: Span,
}

impl WithIdentifier for Argument {
    fn identifier(&self) -> &Identifier {
        &self.name
    }
}

impl Argument {
    pub fn new_string(name: &str, value: &str) -> Argument {
        Argument {
            name: Identifier::new(name),
            value: Value::StringValue(String::from(value), Span::empty()),
            span: Span::empty(),
        }
    }

    pub fn new_constant(name: &str, value: &str) -> Argument {
        Argument {
            name: Identifier::new(name),
            value: Value::ConstantValue(String::from(value), Span::empty()),
            span: Span::empty(),
        }
    }

    pub fn new_array(name: &str, value: Vec<Value>) -> Argument {
        Argument {
            name: Identifier::new(name),
            value: Value::Array(value, Span::empty()),
            span: Span::empty(),
        }
    }

    pub fn new_function(name: &str, fn_name: &str, value: Vec<Value>) -> Argument {
        Argument {
            name: Identifier::new(name),
            value: Value::Function(fn_name.to_string(), value, Span::empty()),
            span: Span::empty(),
        }
    }

    pub fn new(name: &str, value: Value) -> Argument {
        Argument {
            name: Identifier::new(name),
            value,
            span: Span::empty(),
        }
    }
}
