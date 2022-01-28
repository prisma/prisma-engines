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

/// An attribute (with @ or @@).
#[derive(Debug, Clone, Copy)]
pub enum AttributeId {
    /// On a model
    Model(super::ModelId, usize),
    /// On a model field.
    ModelField(super::ModelId, super::FieldId, usize),
    /// Attributes in a type alias.
    TypeAlias(super::AliasId, usize),
    /// On a composite type field.
    CompositeTypeField(super::CompositeTypeId, super::FieldId, usize),
    /// On an enum
    Enum(super::EnumId, usize),
    /// On an enum value
    EnumValue(super::EnumId, usize, usize),
}

impl std::ops::Index<AttributeId> for super::SchemaAst {
    type Output = Attribute;

    fn index(&self, index: AttributeId) -> &Self::Output {
        match index {
            AttributeId::ModelField(model_id, field_id, attribute_idx) => {
                &self[model_id][field_id].attributes[attribute_idx]
            }
            AttributeId::TypeAlias(alias_id, attribute_idx) => &self[alias_id].attributes[attribute_idx],
            AttributeId::CompositeTypeField(ctid, field_id, attribute_idx) => {
                &self[ctid][field_id].attributes[attribute_idx]
            }
            AttributeId::Model(model_id, attribute_idx) => &self[model_id].attributes[attribute_idx],
            AttributeId::Enum(enum_id, attribute_idx) => &self[enum_id].attributes[attribute_idx],
            AttributeId::EnumValue(enum_id, enum_value_idx, attribute_idx) => {
                &self[enum_id].values[enum_value_idx].attributes[attribute_idx]
            }
        }
    }
}
