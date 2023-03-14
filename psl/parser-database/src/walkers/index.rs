use either::Either;

use super::CompositeTypeFieldWalker;
use crate::{
    ast,
    types::{IndexAlgorithm, IndexAttribute},
    walkers::{ModelWalker, ScalarFieldAttributeWalker, ScalarFieldWalker},
    ParserDatabase, ScalarFieldType,
};

/// An index, unique or fulltext attribute.
#[derive(Copy, Clone)]
pub struct IndexWalker<'db> {
    pub(crate) model_id: ast::ModelId,
    pub(crate) index: ast::AttributeId,
    pub(crate) db: &'db ParserDatabase,
    pub(crate) index_attribute: &'db IndexAttribute,
}

impl<'db> IndexWalker<'db> {
    /// The mapped name of the index.
    ///
    /// ```ignore
    /// @@index([a, b], map: "theName")
    ///                      ^^^^^^^^^
    /// ```
    pub fn mapped_name(self) -> Option<&'db str> {
        self.index_attribute.mapped_name.map(|id| &self.db[id])
    }

    /// The attribute name: `"index"` for `@@unique`, `"fulltext"` for `@@fultext` and `"index"`
    /// for `@index` and `@@index`.
    pub fn attribute_name(self) -> &'static str {
        if self.is_unique() && self.is_defined_on_field() {
            "@unique"
        } else if self.is_unique() {
            "@@unique"
        } else if self.is_fulltext() {
            "@@fulltext"
        } else {
            "@@index"
        }
    }

    /// The id of the index, if explicitly defined.
    pub fn attribute_id(self) -> ast::AttributeId {
        self.index
    }

    /// The index type.
    pub fn index_type(self) -> crate::types::IndexType {
        self.attribute().r#type
    }

    /// The `name` argument of the index attribute. The client name.
    ///
    /// ```ignore
    /// @@index([a, b], name: "theName")
    ///                      ^^^^^^^^^
    /// ```
    pub fn name(self) -> Option<&'db str> {
        self.index_attribute.name.map(|id| &self.db[id])
    }

    /// The index algorithm, if a specific one was specified for the index.
    pub fn algorithm(self) -> Option<IndexAlgorithm> {
        self.attribute().algorithm
    }

    /// The AST node of the index/unique attribute.
    pub fn ast_attribute(self) -> &'db ast::Attribute {
        &self.db.ast[self.index]
    }

    pub(crate) fn attribute(self) -> &'db IndexAttribute {
        self.index_attribute
    }

    fn fields_array(self) -> &'db [ast::Expression] {
        self.ast_attribute()
            .arguments
            .arguments
            .iter()
            .find(|arg| match &arg.name {
                Some(ident) if ident.name.is_empty() || ident.name == "fields" => true,
                None => true,
                Some(_) => false,
            })
            .and_then(|arg| arg.value.as_array())
            .unwrap()
            .0
    }

    /// Iterate over all the names in all the paths in the fields argument.
    ///
    /// For example, `@@index([a, b.c.d])` would return an iterator over "a", "b", "c", "d".
    pub fn all_field_names(self) -> impl Iterator<Item = &'db str> {
        self.fields_array()
            .iter()
            .map(|path| match path {
                ast::Expression::ConstantValue(name, _) => name,
                ast::Expression::Function(name, _, _) => name,
                _ => unreachable!(),
            })
            .flat_map(|name| name.split('.'))
    }

    /// The scalar fields covered by the index.
    pub fn fields(self) -> impl ExactSizeIterator<Item = IndexFieldWalker<'db>> {
        self.index_attribute
            .fields
            .iter()
            .map(move |attributes| match attributes.path.field_in_index() {
                Either::Left(ctid) => IndexFieldWalker::new(self.db.walk(ctid)),
                Either::Right(id) => IndexFieldWalker::new(self.db.walk(id)),
            })
    }

    /// The scalar fields covered by the index, and their arguments.
    pub fn scalar_field_attributes(self) -> impl ExactSizeIterator<Item = ScalarFieldAttributeWalker<'db>> {
        self.attribute()
            .fields
            .iter()
            .enumerate()
            .map(move |(field_arg_id, _)| ScalarFieldAttributeWalker {
                fields: &self.attribute().fields,
                db: self.db,
                field_arg_id,
            })
    }

    /// True, if given field is a part of the indexed fields.
    pub fn contains_field(self, field: ScalarFieldWalker<'db>) -> bool {
        self.fields().filter_map(|f| f.as_scalar_field()).any(|f| f == field)
    }

    /// True if the field contains exactly the same fields in the same order,
    /// and with the same attributes.
    pub fn contains_exactly_the_fields(
        self,
        fields: impl ExactSizeIterator<Item = ScalarFieldAttributeWalker<'db>>,
    ) -> bool {
        if self.scalar_field_attributes().len() != fields.len() {
            return false;
        }

        self.scalar_field_attributes().zip(fields).all(|(a, b)| {
            let same_attributes = a.sort_order() == b.sort_order() && a.length() == b.length();
            let same_path = a.as_path_to_indexed_field() == b.as_path_to_indexed_field();

            same_path && same_attributes
        })
    }

    /// Whether the index is defined on a single field (otherwise: on the model).
    pub fn is_defined_on_field(self) -> bool {
        self.index_attribute.source_field.is_some()
    }

    /// Is this an `@@unique`?
    pub fn is_unique(self) -> bool {
        self.index_attribute.is_unique()
    }

    /// Is this a `@@fulltext`?
    pub fn is_fulltext(self) -> bool {
        self.index_attribute.is_fulltext()
    }

    /// Is this an `@@index`?
    pub fn is_normal(self) -> bool {
        self.index_attribute.is_normal()
    }

    /// If true, the index defines the storage and ordering of the row. Mostly
    /// matters on SQL Server where one can change the clustering.
    pub fn clustered(self) -> Option<bool> {
        self.index_attribute.clustered
    }

    /// The model the index is defined on.
    pub fn model(self) -> ModelWalker<'db> {
        self.db.walk(self.model_id)
    }

    /// The field the model was defined on, if any.
    pub fn source_field(self) -> Option<ScalarFieldWalker<'db>> {
        self.index_attribute.source_field.map(|field_id| self.db.walk(field_id))
    }
}

impl<'db> From<ScalarFieldWalker<'db>> for IndexFieldWalker<'db> {
    fn from(sf: ScalarFieldWalker<'db>) -> Self {
        Self::Scalar(sf)
    }
}

impl<'db> From<CompositeTypeFieldWalker<'db>> for IndexFieldWalker<'db> {
    fn from(cf: CompositeTypeFieldWalker<'db>) -> Self {
        Self::Composite(cf)
    }
}

/// A field in an index definition. It can point to a scalar field in the
/// current model, or through embedding a field in a composite type.
#[derive(Copy, Clone, PartialEq)]
pub enum IndexFieldWalker<'db> {
    /// A field on a model.
    Scalar(ScalarFieldWalker<'db>),
    /// The path to a field in a composite type.
    Composite(CompositeTypeFieldWalker<'db>),
}

impl<'db> IndexFieldWalker<'db> {
    pub(super) fn new(inner: impl Into<IndexFieldWalker<'db>>) -> Self {
        inner.into()
    }

    /// Is the field optional / nullable?
    pub fn is_optional(self) -> bool {
        match self {
            IndexFieldWalker::Scalar(sf) => sf.is_optional(),
            IndexFieldWalker::Composite(cf) => cf.arity().is_optional(),
        }
    }

    /// Is the field a list?
    pub fn is_list(self) -> bool {
        match self {
            IndexFieldWalker::Scalar(sf) => sf.is_list(),
            IndexFieldWalker::Composite(cf) => cf.arity().is_list(),
        }
    }

    /// Is the type of the field `Unsupported("...")`?
    pub fn is_unsupported(self) -> bool {
        match self {
            IndexFieldWalker::Scalar(sf) => sf.is_unsupported(),
            IndexFieldWalker::Composite(cf) => cf.r#type().is_unsupported(),
        }
    }

    /// The ID of the field node in the AST.
    pub fn field_id(self) -> ast::FieldId {
        match self {
            IndexFieldWalker::Scalar(sf) => sf.field_id(),
            IndexFieldWalker::Composite(cf) => cf.field_id(),
        }
    }

    /// The name of the field.
    pub fn name(self) -> &'db str {
        match self {
            IndexFieldWalker::Scalar(sf) => sf.name(),
            IndexFieldWalker::Composite(cf) => cf.name(),
        }
    }

    /// The final database name of the field. See crate docs for explanations on database names.
    pub fn database_name(self) -> &'db str {
        match self {
            IndexFieldWalker::Scalar(sf) => sf.database_name(),
            IndexFieldWalker::Composite(cf) => cf.database_name(),
        }
    }

    /// The type of the field.
    pub fn scalar_field_type(self) -> ScalarFieldType {
        match self {
            IndexFieldWalker::Scalar(sf) => sf.scalar_field_type(),
            IndexFieldWalker::Composite(cf) => cf.r#type(),
        }
    }

    /// Convert the walker to a scalar field, if the underlying field is in a
    /// model.
    pub fn as_scalar_field(self) -> Option<ScalarFieldWalker<'db>> {
        match self {
            IndexFieldWalker::Scalar(sf) => Some(sf),
            IndexFieldWalker::Composite(_) => None,
        }
    }

    /// Convert the walker to a composite field, if the underlying field is in a
    /// composite type.
    pub fn as_composite_field(self) -> Option<CompositeTypeFieldWalker<'db>> {
        match self {
            IndexFieldWalker::Scalar(_) => None,
            IndexFieldWalker::Composite(cf) => Some(cf),
        }
    }

    /// True if the index field is a scalar field.
    pub fn is_scalar_field(self) -> bool {
        matches!(self, IndexFieldWalker::Scalar(_))
    }

    /// True if the index field is a composite field.
    pub fn is_composite_field(self) -> bool {
        matches!(self, IndexFieldWalker::Composite(_))
    }

    /// Does the field define a primary key by its own.
    pub fn is_single_pk(self) -> bool {
        match self {
            IndexFieldWalker::Scalar(sf) => sf.is_single_pk(),
            IndexFieldWalker::Composite(_) => false,
        }
    }

    /// (attribute scope, native type name, arguments, span)
    ///
    /// For example: `@db.Text` would translate to ("db", "Text", &[], <the span>)
    pub fn raw_native_type(self) -> Option<(&'db str, &'db str, &'db [String], ast::Span)> {
        match self {
            IndexFieldWalker::Scalar(sf) => sf.raw_native_type(),
            IndexFieldWalker::Composite(cf) => cf.raw_native_type(),
        }
    }

    /// The field node in the AST.
    pub fn ast_field(self) -> &'db ast::Field {
        match self {
            IndexFieldWalker::Scalar(sf) => sf.ast_field(),
            IndexFieldWalker::Composite(cf) => cf.ast_field(),
        }
    }
}
