use crate::{
    ast,
    types::{IndexAlgorithm, IndexAttribute},
    walkers::{ModelWalker, ScalarFieldAttributeWalker, ScalarFieldWalker},
    ParserDatabase,
};

/// An index, unique or fulltext attribute.
#[derive(Copy, Clone)]
pub struct IndexWalker<'ast, 'db> {
    pub(crate) model_id: ast::ModelId,
    pub(crate) index: Option<&'ast ast::Attribute>,
    pub(crate) db: &'db ParserDatabase<'ast>,
    pub(crate) index_attribute: &'db IndexAttribute<'ast>,
}

impl<'ast, 'db> IndexWalker<'ast, 'db> {
    /// The mapped name of the index.
    ///
    /// ```ignore
    /// @@index([a, b], map: "theName")
    ///                      ^^^^^^^^^
    /// ```
    pub fn mapped_name(self) -> Option<&'ast str> {
        self.index_attribute.db_name
    }

    /// The attribute name: `"unique"` for `@@unique`, `"fulltext"` for `@@fultext` and `"index"`
    /// for `@index` and `@@index`.
    pub fn attribute_name(self) -> &'static str {
        if self.is_unique() {
            "unique"
        } else {
            "index"
        }
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
    pub fn name(self) -> Option<&'ast str> {
        self.index_attribute.name
    }

    /// The index algorithm, if a specific one was specified for the index.
    pub fn algorithm(self) -> Option<IndexAlgorithm> {
        self.attribute().algorithm
    }

    /// The AST node of the index/unique attribute.
    pub fn ast_attribute(self) -> Option<&'ast ast::Attribute> {
        self.index
    }

    pub(crate) fn attribute(self) -> &'db IndexAttribute<'ast> {
        self.index_attribute
    }

    /// The scalar fields covered by the index.
    pub fn fields(self) -> impl ExactSizeIterator<Item = ScalarFieldWalker<'ast, 'db>> + 'db {
        self.index_attribute
            .fields
            .iter()
            .map(move |field_id| ScalarFieldWalker {
                model_id: self.model_id,
                field_id: field_id.field_id,
                db: self.db,
                scalar_field: &self.db.types.scalar_fields[&(self.model_id, field_id.field_id)],
            })
    }

    /// The scalar fields covered by the index, and their arguments.
    pub fn scalar_field_attributes(self) -> impl ExactSizeIterator<Item = ScalarFieldAttributeWalker<'ast, 'db>> + 'db {
        self.attribute()
            .fields
            .iter()
            .enumerate()
            .map(move |(field_arg_id, _)| ScalarFieldAttributeWalker {
                model_id: self.model_id,
                fields: &self.attribute().fields,
                db: self.db,
                field_arg_id,
            })
    }

    pub(crate) fn contains_exactly_fields(
        self,
        fields: impl ExactSizeIterator<Item = ScalarFieldWalker<'ast, 'db>>,
    ) -> bool {
        self.index_attribute.fields.len() == fields.len() && self.fields().zip(fields).all(|(a, b)| a == b)
    }

    /// Whether the index is defined on a single field (otherwise: on the model).
    pub fn is_defined_on_field(self) -> bool {
        self.index_attribute.source_field.is_some()
    }

    /// Is this an `@@unique`?
    pub fn is_unique(self) -> bool {
        self.index_attribute.is_unique()
    }

    /// Is this an `@@fulltext`?
    pub fn is_fulltext(self) -> bool {
        self.index_attribute.is_fulltext()
    }

    /// The model the index is defined on.
    pub fn model(self) -> ModelWalker<'ast, 'db> {
        ModelWalker {
            model_id: self.model_id,
            db: self.db,
            model_attributes: &self.db.types.model_attributes[&self.model_id],
        }
    }
}
