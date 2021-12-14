use crate::{walkers::*, ParserDatabase};
use schema_ast::ast;

impl<'ast> ParserDatabase<'ast> {
    /// Gather all the context for a given schema position, expressed as a byte offset.
    pub fn walk_at_position(&self, position: usize) -> SchemaPosition<'ast, '_> {
        match self.ast.find_top_at_position(position) {
            Some(ast::TopId::Model(model_id)) => {
                SchemaPosition::Model(ModelContext::new(self.walk_model(model_id), position))
            }
            // Returning TopLevel as a proxy for "not implemented yet".
            Some(_) => SchemaPosition::TopLevel,
            // If we're not in a top-level item, it means we're in-between top-level items. This is
            // fine and expected.
            None => SchemaPosition::TopLevel,
        }
    }
}

/// A cursor position in a schema.
#[derive(Debug)]
pub enum SchemaPosition<'ast, 'db> {
    /// In-between top-level items
    TopLevel,
    /// In a model
    Model(ModelContext<'ast, 'db>),
}

/// A cursor position in a context.
#[derive(Debug)]
pub enum ModelContext<'ast, 'db> {
    /// In the model, but not somewhere more specific.
    TopLevel(ModelWalker<'ast, 'db>),
    /// In an attribute.
    ModelAttribute,
    /// In a scalar field.
    ScalarField(ScalarFieldContext<'ast, 'db>),
    /// In a relation field.
    RelationField(RelationFieldContext<'ast, 'db>),
}

impl<'ast, 'db> ModelContext<'ast, 'db> {
    fn new(model: ModelWalker<'ast, 'db>, position: usize) -> Self {
        for field in model.scalar_fields() {
            if field.ast_field().span.contains(position) {
                return ModelContext::ScalarField(ScalarFieldContext::new(field, position));
            }
        }

        for field in model.relation_fields() {
            if field.ast_field().span.contains(position) {
                return ModelContext::RelationField(RelationFieldContext::new(field, position));
            }
        }

        ModelContext::TopLevel(model)
    }
}

/// In a relation field
#[derive(Debug)]
pub enum RelationFieldContext<'ast, 'db> {
    /// Nowhere specific inside a relation field.
    Field(RelationFieldWalker<'ast, 'db>),
    /// Inside the relation argument
    RelationAttribute(
        RelationFieldWalker<'ast, 'db>,
        &'ast ast::Attribute,
        Option<&'ast ast::Argument>,
    ),
}

impl<'ast, 'db> RelationFieldContext<'ast, 'db> {
    fn new(field: RelationFieldWalker<'ast, 'db>, position: usize) -> RelationFieldContext<'ast, 'db> {
        if let Some(relation_attr) = field.relation_attribute() {
            if relation_attr.span.contains(position) {
                let argument = relation_attr.arguments.iter().find(|arg| arg.span.contains(position));

                return RelationFieldContext::RelationAttribute(field, relation_attr, argument);
            }
        }

        RelationFieldContext::Field(field)
    }
}

/// In a scalar field.
#[derive(Debug)]
pub enum ScalarFieldContext<'ast, 'db> {
    /// Nowhere specific inside the field
    Field(ScalarFieldWalker<'ast, 'db>),
    /// In the `@default` attribute.
    DefaultAttribute(ScalarFieldWalker<'ast, 'db>, &'ast ast::Attribute),
}

impl<'ast, 'db> ScalarFieldContext<'ast, 'db> {
    fn new(field: ScalarFieldWalker<'ast, 'db>, position: usize) -> Self {
        if let Some(attr) = field.default_attribute() {
            if attr.span.contains(position) && !attr.arguments.iter().any(|arg| arg.span.contains(position)) {
                return ScalarFieldContext::DefaultAttribute(field, attr);
            }
        }

        ScalarFieldContext::Field(field)
    }
}
