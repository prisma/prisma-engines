use super::*;

/// An opaque identifier for a field in an AST model. Use the
/// `model[field_id]` syntax to resolve the id to an `ast::Field`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct FieldId(pub(super) u32);

impl FieldId {
    /// Used for range bounds when iterating over BTreeMaps.
    pub(crate) const ZERO: FieldId = FieldId(0);
    /// Used for range bounds when iterating over BTreeMaps.
    pub(crate) const MAX: FieldId = FieldId(u32::MAX);
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
    pub(crate) fn iter_fields(&self) -> impl Iterator<Item = (FieldId, &Field)> {
        self.fields
            .iter()
            .enumerate()
            .map(|(idx, field)| (FieldId(idx as u32), field))
    }

    pub(crate) fn find_field(&self, name: &str) -> Option<&Field> {
        self.fields.iter().find(|ast_field| ast_field.name.name == name)
    }

    pub(crate) fn find_field_bang(&self, name: &str) -> &Field {
        self.find_field(name).unwrap()
    }

    pub(crate) fn id_attribute(&self) -> &Attribute {
        let from_model = self.attributes().iter().find(|attr| attr.is_id());

        let mut from_field = self
            .iter_fields()
            .flat_map(|(_, field)| field.attributes().iter().find(|attr| attr.is_id()));

        from_model.or_else(|| from_field.next()).unwrap()
    }

    pub(crate) fn name(&self) -> &str {
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
    fn attributes(&self) -> &Vec<Attribute> {
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
