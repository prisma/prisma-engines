use super::model::FieldId;
use super::*;

/// A type declaration.
#[derive(Debug, Clone, PartialEq)]
pub struct TypeDefinition {
    /// The name of the type.
    pub name: Identifier,
    /// The fields of the type.
    pub fields: Vec<Field>,
    /// The documentation for this type.
    pub documentation: Option<Comment>,
    /// The location of this type in the text representation.
    pub span: Span,
    /// Should this be commented out.
    pub commented_out: bool,
}

impl TypeDefinition {
    pub(crate) fn iter_fields(&self) -> impl Iterator<Item = (FieldId, &Field)> {
        self.fields
            .iter()
            .enumerate()
            .map(|(idx, field)| (FieldId(idx as u32), field))
    }

    pub(crate) fn find_field(&self, name: &str) -> Option<&Field> {
        self.fields.iter().find(|ast_field| ast_field.name.name == name)
    }
}

impl WithIdentifier for TypeDefinition {
    fn identifier(&self) -> &Identifier {
        &self.name
    }
}

impl WithSpan for TypeDefinition {
    fn span(&self) -> &Span {
        &self.span
    }
}

impl WithDocumentation for TypeDefinition {
    fn documentation(&self) -> &Option<Comment> {
        &self.documentation
    }

    fn is_commented_out(&self) -> bool {
        self.commented_out
    }
}
