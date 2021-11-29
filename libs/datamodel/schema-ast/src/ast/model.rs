use super::{Attribute, Comment, Field, Identifier, Span, WithAttributes, WithDocumentation, WithIdentifier, WithSpan};

/// An opaque identifier for a field in an AST model. Use the
/// `model[field_id]` syntax to resolve the id to an `ast::Field`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FieldId(pub(super) u32);

impl FieldId {
    /// Used for range bounds when iterating over BTreeMaps.
    pub const ZERO: FieldId = FieldId(0);
    /// Used for range bounds when iterating over BTreeMaps.
    pub const MAX: FieldId = FieldId(u32::MAX);
}

impl std::ops::Index<FieldId> for Model {
    type Output = Field;

    fn index(&self, index: FieldId) -> &Self::Output {
        &self.fields[index.0 as usize]
    }
}

/// A model declaration.
#[derive(Debug, Clone, PartialEq)]
pub struct Model {
    /// The name of the model.
    pub name: Identifier,
    /// The fields of the model.
    pub fields: Vec<Field>,
    /// The attributes of this model.
    pub attributes: Vec<Attribute>,
    /// The documentation for this model.
    pub documentation: Option<Comment>,
    /// The location of this model in the text representation.
    pub span: Span,
    /// Should this be commented out.
    pub commented_out: bool,
}

impl Model {
    pub fn iter_fields(&self) -> impl Iterator<Item = (FieldId, &Field)> {
        self.fields
            .iter()
            .enumerate()
            .map(|(idx, field)| (FieldId(idx as u32), field))
    }

    pub fn find_field(&self, name: &str) -> Option<&Field> {
        self.fields.iter().find(|ast_field| ast_field.name.name == name)
    }

    pub fn find_field_bang(&self, name: &str) -> &Field {
        self.find_field(name).unwrap()
    }

    pub fn name(&self) -> &str {
        &self.name.name
    }
}

impl WithIdentifier for Model {
    fn identifier(&self) -> &Identifier {
        &self.name
    }
}

impl WithSpan for Model {
    fn span(&self) -> &Span {
        &self.span
    }
}

impl WithAttributes for Model {
    fn attributes(&self) -> &[Attribute] {
        &self.attributes
    }
}

impl WithDocumentation for Model {
    fn documentation(&self) -> &Option<Comment> {
        &self.documentation
    }

    fn is_commented_out(&self) -> bool {
        self.commented_out
    }
}
