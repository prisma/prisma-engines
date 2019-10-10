use super::*;

#[derive(Debug)]
pub struct Field {
    /// The field's type.
    pub field_type: Identifier,
    /// The name of the field.
    pub name: Identifier,
    /// The aritiy of the field.
    pub arity: FieldArity,
    /// The default value of the field.
    pub default_value: Option<Expression>,
    /// The directives of this field.
    pub directives: Vec<Directive>,
    /// The comments for this field.
    pub documentation: Option<Comment>,
    /// The location of this field in the text representation.
    pub span: Span,
}

impl WithIdentifier for Field {
    fn identifier(&self) -> &Identifier {
        &self.name
    }
}

impl WithSpan for Field {
    fn span(&self) -> &Span {
        &self.span
    }
}

impl WithDirectives for Field {
    fn directives(&self) -> &Vec<Directive> {
        &self.directives
    }
}

impl WithDocumentation for Field {
    fn documentation(&self) -> &Option<Comment> {
        &self.documentation
    }
}

#[derive(Debug)]
pub enum FieldArity {
    Required,
    Optional,
    List,
}
