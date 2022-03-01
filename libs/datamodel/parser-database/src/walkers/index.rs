use crate::{
    ast,
    types::{IndexAlgorithm, IndexAttribute},
    walkers::{ModelWalker, ScalarFieldAttributeWalker, ScalarFieldWalker},
    ParserDatabase,
};

/// An index, unique or fulltext attribute.
#[derive(Copy, Clone)]
pub struct IndexWalker<'db> {
    pub(crate) model_id: ast::ModelId,
    pub(crate) index: Option<ast::AttributeId>,
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
    pub fn name(self) -> Option<&'db str> {
        self.index_attribute.name.map(|id| &self.db[id])
    }

    /// The index algorithm, if a specific one was specified for the index.
    pub fn algorithm(self) -> Option<IndexAlgorithm> {
        self.attribute().algorithm
    }

    /// The AST node of the index/unique attribute.
    pub fn ast_attribute(self) -> Option<&'db ast::Attribute> {
        self.index.map(|id| &self.db.ast[id])
    }

    pub(crate) fn attribute(self) -> &'db IndexAttribute {
        self.index_attribute
    }

    /// The scalar fields covered by the index.
    pub fn fields(self) -> impl ExactSizeIterator<Item = ScalarFieldWalker<'db>> + 'db {
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
    pub fn scalar_field_attributes(self) -> impl ExactSizeIterator<Item = ScalarFieldAttributeWalker<'db>> + 'db {
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
    pub fn model(self) -> ModelWalker<'db> {
        ModelWalker {
            model_id: self.model_id,
            db: self.db,
            model_attributes: &self.db.types.model_attributes[&self.model_id],
        }
    }
}
