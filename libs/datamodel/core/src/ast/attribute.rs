use super::{Argument, Identifier, Span, WithIdentifier, WithSpan};
use std::ops::Index;

#[derive(Debug, Clone, PartialEq)]
pub struct Attribute {
    pub name: Identifier,
    pub arguments: Vec<Argument>,
    pub span: Span,
}

impl Attribute {
    pub fn new(name: &str, arguments: Vec<Argument>) -> Attribute {
        Attribute {
            name: Identifier::new(name),
            arguments,
            span: Span::empty(),
        }
    }

    pub fn name(&self) -> &str {
        &self.name.name
    }

    pub fn is_index(&self) -> bool {
        matches!(self.name.name.as_str(), "index" | "unique")
    }

    pub fn is_id(&self) -> bool {
        matches!(self.name.name.as_str(), "id")
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

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct AttributeId(u32);

impl Index<AttributeId> for super::Model {
    type Output = Attribute;

    fn index(&self, index: AttributeId) -> &Self::Output {
        &self.attributes[index.0 as usize]
    }
}
