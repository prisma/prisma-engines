use crate::{ast, dml};

/// Implementors of this trait are used for serialization to AST attributes.
pub trait AttributeValidator<T> {
    /// Gets the attribute name.
    fn attribute_name(&self) -> &'static str;

    /// Serializes the given attribute's arguments for rendering.
    fn serialize(&self, obj: &T, datamodel: &dml::Datamodel) -> Vec<ast::Attribute>;
}
