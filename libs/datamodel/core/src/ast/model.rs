use super::*;

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
    pub fn find_field(&self, name: &str) -> &Field {
        self.fields
            .iter()
            .find(|ast_field| ast_field.name.name == name)
            .unwrap()
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
