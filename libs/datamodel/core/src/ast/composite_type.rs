use crate::ast::{Comment, Field, Identifier, SchemaAst, Span};

/// A composite type declaration.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct CompositeType {
    /// The name of the type.
    pub name: Identifier,
    /// The fields of the type.
    pub fields: Vec<Field>,
    /// The documentation for this type.
    pub documentation: Option<Comment>,
    /// The location of this type in the text representation.
    pub span: Span,
}

impl CompositeType {
    pub(crate) fn is_commented_out(&self) -> bool {
        false
    }
}

/// An opaque identifier for a type definition in a schema AST. Use the
/// `schema[type_id]` syntax to resolve the id to an `ast::CompositeType`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct CompositeTypeId(pub(super) u32);

impl std::ops::Index<CompositeTypeId> for SchemaAst {
    type Output = CompositeType;

    fn index(&self, index: CompositeTypeId) -> &Self::Output {
        self.tops[index.0 as usize].as_composite_type().unwrap()
    }
}
