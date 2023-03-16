use crate::ast::{Comment, Field, FieldId, Identifier, SchemaAst, Span};

use super::{WithDocumentation, WithIdentifier};

/// A type declaration in the data model. Defined by a type keyword and a block
/// of fields that can be embedded in a model.
///
/// A composite type has no definition in the database schema, and is completely
/// a Prisma concept. It gives type-safety to dynamic data such as JSON.
#[derive(Debug, Clone)]
pub struct CompositeType {
    /// The name of the type.
    ///
    /// ```ignore
    /// type Foo { .. }
    ///      ^^^
    /// ```
    pub(crate) name: Identifier,
    /// The fields of the type.
    ///
    /// ```ignore
    /// type Foo {
    ///   bar String
    ///   ^^^^^^^^^^
    /// }
    /// ```
    pub(crate) fields: Vec<Field>,
    /// The documentation for this type.
    ///
    /// ```ignore
    /// /// Lorem ipsum
    ///     ^^^^^^^^^^^
    /// type Foo {
    ///   bar String
    /// }
    /// ```
    pub(crate) documentation: Option<Comment>,
    /// The location of this type in the text representation.
    pub span: Span,
    /// The span of the inner contents.
    pub inner_span: Span,
}

impl CompositeType {
    pub fn is_commented_out(&self) -> bool {
        false
    }

    pub fn iter_fields(&self) -> impl ExactSizeIterator<Item = (FieldId, &Field)> + Clone {
        self.fields
            .iter()
            .enumerate()
            .map(|(idx, field)| (FieldId(idx as u32), field))
    }
}

/// An opaque identifier for a type definition in a schema AST. Use the
/// `schema[type_id]` syntax to resolve the id to an `ast::CompositeType`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CompositeTypeId(pub(super) u32);

impl std::ops::Index<CompositeTypeId> for SchemaAst {
    type Output = CompositeType;

    fn index(&self, index: CompositeTypeId) -> &Self::Output {
        self.tops[index.0 as usize].as_composite_type().unwrap()
    }
}

impl std::ops::Index<FieldId> for CompositeType {
    type Output = Field;

    fn index(&self, index: FieldId) -> &Self::Output {
        &self.fields[index.0 as usize]
    }
}

impl WithDocumentation for CompositeType {
    fn documentation(&self) -> Option<&str> {
        self.documentation.as_ref().map(|doc| doc.text.as_str())
    }
}

impl WithIdentifier for CompositeType {
    fn identifier(&self) -> &Identifier {
        &self.name
    }
}
